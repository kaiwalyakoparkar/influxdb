//! This module contains testing scenarios for Delete

use data_types::chunk_metadata::ChunkId;
use datafusion::logical_plan::{col, lit};
use predicate::predicate::{Predicate, PredicateBuilder};

use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

use server::db::test_helpers::write_lp;
use server::utils::{
    count_mutable_buffer_chunks, count_object_store_chunks, count_read_buffer_chunks, make_db,
};

use super::{DbScenario, DbSetup};

#[derive(Debug)]
/// Setup for delete query test with one table and one chunk moved from MUB to RUB to OS
pub struct DeleteFromMubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteFromMubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the delete predicate is added in MUB and
        // is moved with chunk moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec!["cpu bar=1 10", "cpu bar=2 20"];
        // delete predicate
        let i: f64 = 1.0;
        let expr = col("bar").eq(lit(i));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 15)
            .add_expr(expr)
            .build();

        // delete happens when data in MUB
        let scenario_mub = make_delete_mub(lp_lines.clone(), pred.clone()).await;

        // delete happens when data in MUB then moved to RUB
        let scenario_rub =
            make_delete_mub_to_rub(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens when data in MUB then moved to RUB and then persisted
        let scenario_rub_os = make_delete_mub_to_rub_and_os(
            lp_lines.clone(),
            pred.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB, then persisted, and then RUB is unloaded
        let scenario_os =
            make_delete_mub_to_os(lp_lines.clone(), pred, table_name, partition_key).await;

        // return scenarios to run queries
        vec![scenario_mub, scenario_rub, scenario_rub_os, scenario_os]
    }
}

#[derive(Debug)]
/// Setup for delete query test with one table and one chunk moved from RUB to OS
pub struct DeleteFromRubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteFromRubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the delete predicate is added in RUB
        // and is moved with chunk moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec!["cpu bar=1 10", "cpu bar=2 20"];
        // delete predicate
        let i: f64 = 1.0;
        let expr = col("bar").eq(lit(i));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 15)
            .add_expr(expr)
            .build();

        // delete happens to data in RUB
        let scenario_rub =
            make_delete_rub(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens to data in RUB then persisted
        let scenario_rub_os =
            make_delete_rub_to_os(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens to data in RUB then persisted then RUB unloaded
        let scenario_os =
            make_delete_rub_to_os_and_unload_rub(lp_lines.clone(), pred, table_name, partition_key)
                .await;

        // return scenarios to run queries
        vec![scenario_rub, scenario_rub_os, scenario_os]
    }
}

#[derive(Debug)]
/// Setup for delete query test with one table and one chunk in both RUB and OS
pub struct DeleteFromOsOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteFromOsOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the delete predicate is added to persisted chunks

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec!["cpu bar=1 10", "cpu bar=2 20"];
        // delete predicate
        let i: f64 = 1.0;
        let expr = col("bar").eq(lit(i));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 15)
            .add_expr(expr)
            .build();

        // delete happens after data is persisted but still in RUB
        let scenario_rub_os =
            make_delete_os_with_rub(lp_lines.clone(), pred.clone(), table_name, partition_key)
                .await;

        // delete happens after data is persisted but still in RUB and then unload RUB
        let _scenario_rub_os_unload_rub = make_delete_os_with_rub_then_unload_rub(
            lp_lines.clone(),
            pred.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens after data is persisted and RUB is unloaded
        let _scenario_os = make_delete_os(lp_lines.clone(), pred, table_name, partition_key).await;

        // return scenarios to run queries
        // NGA todo: turn these 2 OS scenarios on. May need to wait for Marco to finish persisting delete predicates first
        // vec![scenario_rub_os, scenario_rub_os_unload_rub, scenario_os]
        vec![scenario_rub_os]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk moved from MUB to RUB to OS
pub struct DeleteMultiExprsFromMubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteMultiExprsFromMubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in MUB and
        // is moved with chunk moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        // delete happens when data in MUB
        let scenario_mub = make_delete_mub(lp_lines.clone(), pred.clone()).await;

        // delete happens when data in MUB then moved to RUB
        let scenario_rub =
            make_delete_mub_to_rub(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens when data in MUB then moved to RUB and then persisted
        let scenario_rub_os = make_delete_mub_to_rub_and_os(
            lp_lines.clone(),
            pred.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB, then persisted, and then RUB is unloaded
        let scenario_os =
            make_delete_mub_to_os(lp_lines.clone(), pred, table_name, partition_key).await;

        // return scenarios to run queries
        vec![scenario_mub, scenario_rub, scenario_rub_os, scenario_os]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk moved from MUB to RUB to OS
pub struct DeleteMultiExprsFromRubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteMultiExprsFromRubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in MUB and
        // is moved with chunk moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        // delete happens to data in RUB
        let scenario_rub =
            make_delete_rub(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens to data in RUB then persisted
        let scenario_rub_os =
            make_delete_rub_to_os(lp_lines.clone(), pred.clone(), table_name, partition_key).await;

        // delete happens to data in RUB then persisted then RUB unloaded
        let scenario_os =
            make_delete_rub_to_os_and_unload_rub(lp_lines.clone(), pred, table_name, partition_key)
                .await;

        // return scenarios to run queries
        vec![scenario_rub, scenario_rub_os, scenario_os]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk moved from MUB to RUB to OS
pub struct DeleteMultiExprsFromOsOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for DeleteMultiExprsFromOsOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in MUB and
        // is moved with chunk moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        // delete happens after data is persisted but still in RUB
        let scenario_rub_os =
            make_delete_os_with_rub(lp_lines.clone(), pred.clone(), table_name, partition_key)
                .await;

        // delete happens after data is persisted but still in RUB and then unload RUB
        let _scenario_rub_os_unload_rub = make_delete_os_with_rub_then_unload_rub(
            lp_lines.clone(),
            pred.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens after data is persisted and RUB is unloaded
        let _scenario_os = make_delete_os(lp_lines.clone(), pred, table_name, partition_key).await;

        // return scenarios to run queries
        // NGA todo: turn these 2 OS scenarios on. May need to wait for Marco to finish persisting delete predicates first
        //vec![scenario_rub_os, scenario_rub_os_unload_rub, scenario_os]
        vec![scenario_rub_os]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk moved from MUB to RUB to OS
/// Two deletes at different chunk stages
pub struct TwoDeleteMultiExprsFromMubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for TwoDeleteMultiExprsFromMubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in MUB and
        // is moved with chunk moving. Then one more delete after moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred1 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        let expr3 = col("bar").not_eq(lit(i));
        let pred2 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(10, 45)
            .add_expr(expr3)
            .build();

        // delete happens when data in MUB
        let scenario_mub =
            make_delete_mub_delete(lp_lines.clone(), pred1.clone(), pred2.clone()).await;

        // delete happens when data in MUB then moved to RUB
        let scenario_rub = make_delete_mub_to_rub_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB and then persisted
        let scenario_rub_os = make_delete_mub_to_rub_and_os_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB, then persisted, and then RUB is unloaded
        let _scenario_os =
            make_delete_mub_to_os_delete(lp_lines.clone(), pred1, pred2, table_name, partition_key)
                .await;

        // return scenarios to run queries
        // NGA todo: BUG in scenario_os
        // vec![scenario_mub, scenario_rub, scenario_rub_os, scenario_os]
        vec![scenario_mub, scenario_rub, scenario_rub_os]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk moved from RUB to OS
/// Two deletes at different chunk stages
pub struct TwoDeleteMultiExprsFromRubOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for TwoDeleteMultiExprsFromRubOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in RUB and
        // is moved with chunk moving. Then one more delete after moving

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred1 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        let expr3 = col("bar").not_eq(lit(i));
        let pred2 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(10, 45)
            .add_expr(expr3)
            .build();

        // delete happens when data in MUB
        let scenario_rub = make_delete_rub_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB
        let scenario_rub_delete = make_delete_rub_to_os_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens when data in MUB then moved to RUB and then persisted
        let _scenario_rub_os = make_delete_rub_to_os_and_unload_rub_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // return scenarios to run queries
        // NGA todo: BUG in scenario_rub_os
        // vec![scenario_rub, scenario_rub_delete, scenario_rub_os]
        vec![scenario_rub, scenario_rub_delete]
    }
}

#[derive(Debug)]
/// Setup for multi-expression delete query test with one table and one chunk in OS
pub struct TwoDeleteMultiExprsFromOsOneMeasurementOneChunk {}
#[async_trait]
impl DbSetup for TwoDeleteMultiExprsFromOsOneMeasurementOneChunk {
    async fn make(&self) -> Vec<DbScenario> {
        // The main purpose of these scenarios is the multi-expression delete predicate is added in OS twice

        // General setup for all scenarios
        let partition_key = "1970-01-01T00";
        let table_name = "cpu";
        // chunk data
        let lp_lines = vec![
            "cpu,foo=me bar=1 10",
            "cpu,foo=you bar=2 20",
            "cpu,foo=me bar=1 30",
            "cpu,foo=me bar=1 40",
        ];
        // delete predicate
        let i: f64 = 1.0;
        let expr1 = col("bar").eq(lit(i));
        let expr2 = col("foo").eq(lit("me"));
        let pred1 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(0, 32)
            .add_expr(expr1)
            .add_expr(expr2)
            .build();

        let expr3 = col("bar").not_eq(lit(i));
        let pred2 = PredicateBuilder::new()
            .table("cpu")
            .timestamp_range(10, 45)
            .add_expr(expr3)
            .build();

        // delete happens after data is persisted but still in RUB
        let scenario_rub_os = make_delete_os_with_rub_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens after data is persisted but still in RUB and then unload RUB
        let _scenario_rub_os_unload_rub = make_delete_os_with_rub_then_unload_rub_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // delete happens after data is persisted and unload RUB
        let _scenario_os = make_delete_os_delete(
            lp_lines.clone(),
            pred1.clone(),
            pred2.clone(),
            table_name,
            partition_key,
        )
        .await;

        // return scenarios to run queries
        // NGA todo: bug scenario_rub_os_unload_rub, scenario_os
        vec![scenario_rub_os]
    }
}

// NGA todo: Add scenarios that have many different types of chunks at the same time (MUB, RUB, OS) when a or several deletes happen

// -----------------------------------------------------------------------------
// Helper functions
async fn make_delete_mub(lp_lines: Vec<&str>, pred: Predicate) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // One open MUB, no RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 1);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 0);
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // Still one but frozen MUB, no RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 1);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data in MUB".into(),
        db,
    }
}

async fn make_delete_mub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // One open MUB, no RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 1);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 0);
    // delete from MUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // Still one but frozen MUB, no RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 1);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 0);
    // delete from frozen MUB
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // Still one frozen MUB, no RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 1);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data from MUB then move and then delete data from frozen MUB"
            .into(),
        db,
    }
}

async fn make_delete_mub_to_rub(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data in MUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // No MUB, one RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data in RUB moved from MUB".into(),
        db,
    }
}

async fn make_delete_mub_to_rub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data from MUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data from RUB
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, one RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data from MUB, then move to RUB, then delete data from RUB again"
            .into(),
        db,
    }
}

async fn make_delete_mub_to_rub_and_os(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data in MUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in RUB and OS".into(),
        db,
    }
}

async fn make_delete_mub_to_rub_and_os_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data in MUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete from RUB and OS
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name:
            "Deleted data from MUB then move to RUB and OS, then delete from RUB and OS again"
                .into(),
        db,
    }
}

async fn make_delete_mub_to_os(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data in MUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS".into(),
        db,
    }
}

async fn make_delete_mub_to_os_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // delete data from MUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // move MUB to RUB and the delete predicate will be automatically included in RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // delete data from OS after RUb is unloaded
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data from MUB, then move to RUB to OS, then unload RUB, and delete from OS again".into(),
        db,
    }
}

async fn make_delete_rub(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data in RUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // No MUB, one RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data in RUB".into(),
        db,
    }
}

async fn make_delete_rub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data from RUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // delete data from RUB again
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, one RUB, no OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 0);

    DbScenario {
        scenario_name: "Deleted data from RUB twice ".into(),
        db,
    }
}

async fn make_delete_rub_to_os(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data in RUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in RUB and then persisted to OS".into(),
        db,
    }
}

async fn make_delete_rub_to_os_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data from RUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete data from RUB & OS
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name:
            "Deleted data in RUB and then persisted to OS then delete once more from RUB and OS"
                .into(),
        db,
    }
}

async fn make_delete_rub_to_os_and_unload_rub(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data in RUB
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in RUB then persisted to OS then RUB unloaded".into(),
        db,
    }
}

async fn make_delete_rub_to_os_and_unload_rub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // delete data from RUB
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // delete data from OS
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name:
            "Deleted data in RUB then persisted to OS then RUB unloaded then delete data from OS"
                .into(),
        db,
    }
}

async fn make_delete_os_with_rub(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS with RUB".into(),
        db,
    }
}

async fn make_delete_os_with_rub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, one RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 1);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Delete twice from OS with RUB".into(),
        db,
    }
}

async fn make_delete_os_with_rub_then_unload_rub(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS only but the delete happens before RUB is unloaded"
            .into(),
        db,
    }
}

async fn make_delete_os_with_rub_then_unload_rub_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // delete again
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS only but the delete happens before RUB is unloaded, then delete one more"
            .into(),
        db,
    }
}

async fn make_delete_os(
    lp_lines: Vec<&str>,
    pred: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred)).await.unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS and the delete happens after RUB is unloaded".into(),
        db,
    }
}

async fn make_delete_os_delete(
    lp_lines: Vec<&str>,
    pred1: Predicate,
    pred2: Predicate,
    table_name: &str,
    partition_key: &str,
) -> DbScenario {
    let db = make_db().await.db;
    // create an open MUB
    write_lp(&db, &lp_lines.join("\n")).await;
    // move MUB to RUB
    db.rollover_partition(table_name, partition_key)
        .await
        .unwrap();
    db.move_chunk_to_read_buffer(table_name, partition_key, ChunkId::new(0))
        .await
        .unwrap();
    // persist RUB and the delete predicate will be automatically included in the OS chunk
    db.persist_partition(
        table_name,
        partition_key,
        Instant::now() + Duration::from_secs(1),
    )
    .await
    .unwrap();
    // remove RUB
    db.unload_read_buffer(table_name, partition_key, ChunkId::new(1))
        .unwrap();
    // delete data after persisted but RUB still available
    db.delete("cpu", Arc::new(pred1)).await.unwrap();
    db.delete("cpu", Arc::new(pred2)).await.unwrap();
    // No MUB, no RUB, one OS
    assert_eq!(count_mutable_buffer_chunks(&db), 0);
    assert_eq!(count_read_buffer_chunks(&db), 0);
    assert_eq!(count_object_store_chunks(&db), 1);

    DbScenario {
        scenario_name: "Deleted data in OS and the two delete happens after RUB is unloaded".into(),
        db,
    }
}
