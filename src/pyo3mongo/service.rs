//! Service
//!

use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, Document};
use mongodb::Collection;
use tokio_stream::StreamExt;

use super::db::MongoClient;
use super::model::{Edge, EdgeDto, FindEdgeByVertexDto, PureId, Vertex, VertexDto};
use super::{Pyo3MongoError, Pyo3MongoResult};

/// The graphService is responsible for creating and deleting vertices and edges.
///
/// A graphService contains two collections:
/// 1. ${cat}_vertex
/// 1. ${cat}_edge
#[allow(dead_code)]
pub struct GraphService {
    client: MongoClient,
    cat: String,
}

#[allow(dead_code)]
impl GraphService {
    pub async fn new(uri: &str, cat: &str) -> Pyo3MongoResult<Self> {
        Ok(GraphService {
            client: MongoClient::new(uri).await?,
            cat: cat.to_owned(),
        })
    }

    /// collection of vertex
    fn collection_vertex(&self) -> Collection<Vertex> {
        self.client
            .collection::<Vertex>(&format!("{}_vertex", self.cat))
    }

    /// collection of edge
    fn collection_edge(&self) -> Collection<Edge> {
        self.client
            .collection::<Edge>(&format!("{}_edge", self.cat))
    }

    /// private method, truncate all collections
    pub(crate) async fn truncate_all(&self) -> Pyo3MongoResult<()> {
        self.collection_vertex().delete_many(doc! {}, None).await?;
        self.collection_edge().delete_many(doc! {}, None).await?;
        Ok(())
    }

    pub async fn create_vertex<'a>(&self, dto: VertexDto<'a>) -> Pyo3MongoResult<Vertex> {
        let insert = self
            .collection_vertex()
            .insert_one(Vertex::from(dto), None)
            .await?;

        let id = insert.inserted_id.as_object_id().unwrap();
        let res = self
            .collection_vertex()
            .find_one(doc! {"_id": id}, None)
            .await?
            .ok_or(Pyo3MongoError::Common("vertex not found"));

        Ok(res?)
    }

    pub async fn get_vertex(&self, id: ObjectId) -> Pyo3MongoResult<Vertex> {
        let res = self
            .collection_vertex()
            .find_one(doc! {"_id": id}, None)
            .await?
            .ok_or(Pyo3MongoError::Common("vertex not found"));

        Ok(res?)
    }

    pub async fn get_vertexes(&self, ids: Vec<ObjectId>) -> Pyo3MongoResult<Vec<Vertex>> {
        let res = self
            .collection_vertex()
            .find(doc! {"_id": doc! {"$in": ids}}, None)
            .await?
            .map(|v| v.unwrap())
            .collect::<Vec<Vertex>>()
            .await;

        Ok(res)
    }

    pub async fn get_all_vertexes(&self) -> Pyo3MongoResult<Vec<Vertex>> {
        let mut cursor = self.collection_vertex().find(None, None).await?;

        let mut res = Vec::new();
        while let Some(doc) = cursor.next().await {
            res.push(doc?);
        }

        Ok(res)
    }

    pub async fn update_vertex<'a>(
        &self,
        id: ObjectId,
        dto: VertexDto<'a>,
    ) -> Pyo3MongoResult<Vertex> {
        let id = doc! {"_id": id};
        let update = doc! {
            "$set": Document::from(&Vertex::from(dto))
        };
        let res = self
            .collection_vertex()
            .find_one_and_update(id, update, None)
            .await?
            .ok_or(Pyo3MongoError::Common("vertex not found"));

        Ok(res?)
    }

    /// look up source & target vertexes whether existed
    async fn check_edge_legitimacy<'a>(&self, dto: &EdgeDto<'a>) -> Pyo3MongoResult<()> {
        // make sure source vertex existed
        self.get_vertex(dto.source).await?;

        // make sure target vertex existed
        self.get_vertex(dto.target).await?;

        Ok(())
    }

    pub async fn create_edge<'a>(&self, dto: EdgeDto<'a>) -> Pyo3MongoResult<Edge> {
        if let Err(e) = self.check_edge_legitimacy(&dto).await {
            return Err(e);
        }

        let insert = self
            .collection_edge()
            .insert_one(Edge::from(dto), None)
            .await?;

        let res = self
            .collection_edge()
            .find_one(doc! {"_id": insert.inserted_id}, None)
            .await?
            .ok_or(Pyo3MongoError::Common("edge not found"));

        Ok(res?)
    }

    pub async fn get_edge(&self, id: ObjectId) -> Pyo3MongoResult<Edge> {
        let res = self
            .collection_edge()
            .find_one(doc! {"_id": id}, None)
            .await?
            .ok_or(Pyo3MongoError::Common("edge not fount"));

        Ok(res?)
    }

    pub async fn get_edges(&self, ids: Vec<ObjectId>) -> Pyo3MongoResult<Vec<Edge>> {
        let res = self
            .collection_edge()
            .find(doc! {"_id": doc! {"$in": ids}}, None)
            .await?
            .map(|v| v.unwrap())
            .collect::<Vec<Edge>>()
            .await;

        Ok(res)
    }

    pub async fn get_all_edges(&self) -> Pyo3MongoResult<Vec<Edge>> {
        let mut cursor = self.collection_edge().find(None, None).await?;

        let mut res = Vec::new();
        while let Some(doc) = cursor.next().await {
            res.push(doc?);
        }

        Ok(res)
    }

    pub async fn update_edge<'a>(&self, id: ObjectId, dto: EdgeDto<'a>) -> Pyo3MongoResult<Edge> {
        if let Err(e) = self.check_edge_legitimacy(&dto).await {
            return Err(e);
        }

        let id = doc! {"_id": id};
        let update = doc! {
            "$set": Document::from(&Edge::from(dto))
        };
        let res = self
            .collection_edge()
            .find_one_and_update(id, update, None)
            .await?
            .ok_or(Pyo3MongoError::Common("edge not found"));

        Ok(res?)
    }

    pub async fn delete_edge(&self, id: ObjectId) -> Pyo3MongoResult<()> {
        let res = self
            .collection_edge()
            .delete_one(doc! {"_id": id}, None)
            .await?;

        if res.deleted_count == 0 {
            return Err(Pyo3MongoError::Common("edge not found"));
        }

        Ok(())
    }

    pub async fn delete_edges(&self, ids: Vec<ObjectId>) -> Pyo3MongoResult<()> {
        let res = self
            .collection_edge()
            .delete_many(doc! {"_id": { "$in": ids }}, None)
            .await?;

        if res.deleted_count == 0 {
            return Err(Pyo3MongoError::Common("edge not found"));
        }

        Ok(())
    }

    fn edges_by_vertex_pipeline(&self, find_dto: FindEdgeByVertexDto) -> Vec<Document> {
        // match object id in vertex collection
        let match_id = |id: ObjectId| doc! {"$match": {"_id": id}};
        // from edge collection
        let from = format!("{}_edge", self.cat);
        // lookup related edges, source/target/both
        let lookup = |field: &str| {
            doc! {"$lookup": {
                "from": &from,
                "localField": "_id",
                "foreignField": field,
                "as": "edges"
            }}
        };
        // turn aggregations into a vector of edges document
        let unwind = doc! {"$unwind": "$edges"};
        let replace = doc! {"$replaceRoot": {"newRoot": "$edges"}};

        match find_dto {
            FindEdgeByVertexDto::Source(id) => {
                vec![match_id(id), lookup("source"), unwind, replace]
            }
            FindEdgeByVertexDto::Target(id) => {
                vec![match_id(id), lookup("target"), unwind, replace]
            }
            FindEdgeByVertexDto::Bidirectional(id) => {
                let advanced_lookup = doc! {
                    "$lookup": {
                        "from": &from,
                        "pipeline": [
                            {"$match": {"$expr": {"$or": [{"target": id}, {"source": id}]}}}
                        ],
                        "as": "edges"
                    }
                };

                vec![match_id(id), advanced_lookup, unwind, replace]
            }
        }
    }

    /// get all related edges
    pub async fn get_edges_by_vertex(
        &self,
        find_dto: FindEdgeByVertexDto,
    ) -> Pyo3MongoResult<Vec<Edge>> {
        let pipeline = self.edges_by_vertex_pipeline(find_dto);

        // cursor, futures iterator
        let mut cursor = self.collection_vertex().aggregate(pipeline, None).await?;

        let mut res = Vec::new();
        while let Some(doc) = cursor.next().await {
            let edge: Edge = bson::from_document(doc?)?;
            res.push(edge);
        }

        Ok(res)
    }

    /// delete vertex
    /// atomically delete all related edges and then delete vertex
    pub async fn delete_vertex(&self, id: ObjectId) -> Pyo3MongoResult<()> {
        // get all related edges
        // TODO: do not use `edges_by_vertex_pipeline` here
        let mut pipeline = self.edges_by_vertex_pipeline(FindEdgeByVertexDto::Bidirectional(id));
        // project only _id
        pipeline.push(doc! {"$project": doc! {"_id": 1}});
        // look up all related edges' _id
        let mut cursor = self.collection_vertex().aggregate(pipeline, None).await?;
        let mut ids = Vec::new();
        while let Some(doc) = cursor.next().await {
            let pi: PureId = bson::from_document(doc?)?;
            ids.push(pi.id);
        }

        dbg!(&ids);

        // delete all related edges
        if !ids.is_empty() {
            self.delete_edges(ids).await?;
        }

        // delete vertex
        let delete_vertex = self
            .collection_vertex()
            .delete_one(doc! {"_id": id}, None)
            .await?;

        if delete_vertex.deleted_count == 0 {
            return Err(Pyo3MongoError::Common("edge not found"));
        }

        Ok(())
    }

    pub async fn get_edges_from_vertex_by_label(
        &self,
        vertex_id: ObjectId,
        label: Option<&str>,
        depth: Option<i32>,
    ) -> Pyo3MongoResult<Vec<Edge>> {
        let depth = match depth {
            Some(n) => doc! {"maxDepth": n},
            None => doc! {},
        };
        let restrict = match label {
            Some(l) => doc! {"restrictSearchWithMatch": {"label": l}},
            None => doc! {},
        };
        let mut graph_lookup = doc! {
            "from": format!("{}_edge", self.cat),
            "startWith": "$_id",
            "connectFromField": "target",
            "connectToField": "source",
            "as": "edges",
        };
        graph_lookup.extend(depth);
        graph_lookup.extend(restrict);

        let pipeline = vec![
            doc! {"$match": doc! {"_id": vertex_id}},
            doc! {"$graphLookup": graph_lookup},
            doc! {"$unwind": "$edges"},
            doc! {"$replaceRoot": {"newRoot": "$edges"}},
        ];

        let mut cursor = self.collection_vertex().aggregate(pipeline, None).await?;

        let mut res = Vec::new();
        while let Some(doc) = cursor.next().await {
            let edge: Edge = bson::from_document(doc?)?;
            res.push(edge);
        }

        Ok(res)
    }

    pub async fn get_graph_from_vertex_by_label(
        &self,
        vertex_id: ObjectId,
        label: Option<&str>,
        depth: Option<i32>,
    ) -> Pyo3MongoResult<(Vec<Edge>, Vec<Vertex>)> {
        let edges = self
            .get_edges_from_vertex_by_label(vertex_id, label, depth)
            .await?;

        let target_ids = edges.iter().map(|e| e.target).collect::<Vec<_>>();
        let vertexes = self.get_vertexes(target_ids).await?;

        Ok((edges, vertexes))
    }
}

#[cfg(test)]
mod test_service {

    use super::super::model::{EdgeDto, VertexDto};
    use super::*;

    const URI: &str = "mongodb://root:secret@localhost:27017";
    const CAT: &str = "dev";

    const LABEL: &str = "test-label";

    #[tokio::test]
    async fn test_vertex_crud() {
        let gs = GraphService::new(URI, CAT).await.unwrap();

        let create = gs.create_vertex(VertexDto::new("node-1")).await.unwrap();
        println!("{:?}", create);

        let id = create.id.unwrap();
        let get = gs.get_vertex(id).await.unwrap();
        println!("{:?}", get);

        assert_eq!(create, get);

        let update = gs
            .update_vertex(id, VertexDto::new("node-2"))
            .await
            .unwrap();
        let get = gs.get_vertex(id).await.unwrap();

        // name has been changed
        assert_ne!(update, get);

        let delete = gs.delete_vertex(id).await;
        assert!(delete.is_ok());
    }

    #[tokio::test]
    async fn test_truncate_all() {
        let gs = GraphService::new(URI, CAT).await.unwrap();

        let res = gs.truncate_all().await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_edge_circuit() {
        let gs = GraphService::new(URI, CAT).await.unwrap();

        let node1 = gs.create_vertex(VertexDto::new("node-1")).await.unwrap();
        let node2 = gs.create_vertex(VertexDto::new("node-2")).await.unwrap();
        let node3 = gs.create_vertex(VertexDto::new("node-3")).await.unwrap();

        // node1 -> node2
        gs.create_edge(EdgeDto::new(
            node1.id.unwrap(),
            node2.id.unwrap(),
            Some(1.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node2 -> node3
        gs.create_edge(EdgeDto::new(
            node2.id.unwrap(),
            node3.id.unwrap(),
            Some(2.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node3 -> node1
        gs.create_edge(EdgeDto::new(
            node3.id.unwrap(),
            node1.id.unwrap(),
            Some(3.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        let (edges, vertexes) = gs
            .get_graph_from_vertex_by_label(node1.id.unwrap(), None, None)
            .await
            .unwrap();

        assert_eq!(edges.len(), 3);
        assert_eq!(vertexes.len(), 3);
    }

    #[tokio::test]
    async fn test_edge_crud() {
        let gs = GraphService::new(URI, CAT).await.unwrap();

        let node1 = gs.create_vertex(VertexDto::new("node-1")).await.unwrap();
        let node2 = gs.create_vertex(VertexDto::new("node-2")).await.unwrap();
        let node3 = gs.create_vertex(VertexDto::new("node-3")).await.unwrap();
        let node4 = gs.create_vertex(VertexDto::new("node-4")).await.unwrap();
        let node5 = gs.create_vertex(VertexDto::new("node-5")).await.unwrap();
        let node6 = gs.create_vertex(VertexDto::new("node-6")).await.unwrap();
        let node7 = gs.create_vertex(VertexDto::new("node-7")).await.unwrap();
        let node8 = gs.create_vertex(VertexDto::new("node-8")).await.unwrap();
        let node9 = gs.create_vertex(VertexDto::new("node-9")).await.unwrap();

        // node1 -> node2
        gs.create_edge(EdgeDto::new(
            node1.id.unwrap(),
            node2.id.unwrap(),
            Some(2.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node1 -> node3
        let edge2 = gs
            .create_edge(EdgeDto::new(
                node1.id.unwrap(),
                node3.id.unwrap(),
                Some(3.0),
                Some(LABEL),
            ))
            .await
            .unwrap();

        // node2 -> node3
        let edge2update = gs
            .update_edge(
                edge2.id.unwrap(),
                EdgeDto::new(node2.id.unwrap(), node3.id.unwrap(), Some(3.0), Some(LABEL)),
            )
            .await;
        assert!(edge2update.is_ok());

        // this will return edge1 and edge2
        let edges = gs
            .get_edges_from_vertex_by_label(node1.id.unwrap(), Some(LABEL), None)
            .await
            .unwrap();
        assert_eq!(edges.len(), 2);

        // node1 -> node4
        gs.create_edge(EdgeDto::new(
            node1.id.unwrap(),
            node4.id.unwrap(),
            Some(4.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node1 -> node5
        gs.create_edge(EdgeDto::new(
            node1.id.unwrap(),
            node5.id.unwrap(),
            Some(5.0),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node5 -> node4
        gs.create_edge(EdgeDto::new(
            node5.id.unwrap(),
            node4.id.unwrap(),
            Some(2.1),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node5 -> node6
        gs.create_edge(EdgeDto::new(
            node5.id.unwrap(),
            node6.id.unwrap(),
            Some(2.2),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node7 -> node1
        gs.create_edge(EdgeDto::new(
            node7.id.unwrap(),
            node1.id.unwrap(),
            Some(4.3),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node8 -> node7
        gs.create_edge(EdgeDto::new(
            node8.id.unwrap(),
            node7.id.unwrap(),
            Some(4.4),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node8 -> node2
        gs.create_edge(EdgeDto::new(
            node8.id.unwrap(),
            node2.id.unwrap(),
            Some(4.5),
            Some(LABEL),
        ))
        .await
        .unwrap();

        // node9 -> node3
        gs.create_edge(EdgeDto::new(
            node9.id.unwrap(),
            node3.id.unwrap(),
            Some(4.6),
            Some(LABEL),
        ))
        .await
        .unwrap();

        /*
        n8 -> n7 -> n1 -> n2 -> n3
        n8 -> n2
        n1 -> n4
        n1 -> n5 -> n4
        n5 -> n6
        n9 -> n3
        */
        let (edges, vertexes) = gs
            .get_graph_from_vertex_by_label(node1.id.unwrap(), None, None)
            .await
            .unwrap();
        assert_eq!(edges.len(), 6);
        assert_eq!(vertexes.len(), 5);

        // TODO: BUG
        // delete node2, related edges should be deleted: n8 -> n2, n1 -> n2, n2 -> n3
        // let delete_n2 = gs.delete_vertex(node2.id.unwrap()).await;
        // assert!(delete_n2.is_ok());

        // // node1 graph
        // let (edges, vertexes) = gs
        //     .get_graph_from_vertex_by_label(node1.id.unwrap(), None, None)
        //     .await
        //     .unwrap();
        // assert_eq!(edges.len(), 4);
        // assert_eq!(vertexes.len(), 3);

        // // node8 graph
        // let (edges, vertexes) = gs
        //     .get_graph_from_vertex_by_label(node8.id.unwrap(), None, None)
        //     .await
        //     .unwrap();
        // assert_eq!(edges.len(), 6);
        // assert_eq!(vertexes.len(), 5);

        // // delete node1, related edges should be deleted: n7 -> n1, n1 -> n4, n1 -> n5
        // let delete_n1 = gs.delete_vertex(node1.id.unwrap()).await;
        // assert!(delete_n1.is_ok());

        // // node8 graph
        // let (edges, vertexes) = gs
        //     .get_graph_from_vertex_by_label(node8.id.unwrap(), None, None)
        //     .await
        //     .unwrap();
        // assert_eq!(edges.len(), 1);
        // assert_eq!(vertexes.len(), 1);
    }
}
