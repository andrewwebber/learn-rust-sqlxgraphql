#[macro_use]
extern crate sqlx;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::*;
use async_graphql_warp::{graphql_subscription_with_data, GQLResponse};
use sqlx::postgres::{PgPoolOptions, Postgres};
use sqlx::Pool;
use std::convert::Infallible;
use std::env;
use warp::{http::Response, Filter, Rejection};

#[SimpleObject]
#[derive(FromRow, Debug)]
struct User {
    age: i32,
    name: String,
}

struct QueryUsers;

#[Object]
impl QueryUsers {
    async fn users(&self, ctx: &Context<'_>) -> FieldResult<Vec<User>> {
        let pool = ctx.data::<std::sync::Arc<Pool<Postgres>>>()?;

        let output = sqlx::query_as::<_, User>(
            "
SELECT age,name
FROM users
        ",
        )
        .fetch_all(pool.as_ref()) // -> Vec<Country>
        .await?;

        Ok(output.into_iter().collect())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), String> {
    let db_url = env::var("DATABASE_URL").expect("unable to find env variable DATABASE_URL");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| format!("{:?}", e))?;
    let pool_arc = std::sync::Arc::new(pool);
    let schema = Schema::build(QueryUsers, EmptyMutation, EmptySubscription)
        .data(pool_arc.clone())
        .finish();

    println!("Playground: http://localhost:8000");

    let graphql_post = async_graphql_warp::graphql(schema).and_then(
        |(schema, builder): (_, QueryBuilder)| async move {
            let resp = builder.execute(&schema).await;
            Ok::<_, Infallible>(GQLResponse::from(resp))
        },
    );

    let graphql_playground = warp::path::end().and(warp::get()).map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/")))
    });

    let routes = graphql_playground.or(graphql_post);

    let warp_server = warp::serve(routes).run(([0, 0, 0, 0], 8000));
    let signal_detected = tokio::signal::ctrl_c();

    tokio::select! {
        _ = warp_server => println!("warp server"),
        _ = signal_detected => println!("signal_detected"),
    }

    Ok(())
}
