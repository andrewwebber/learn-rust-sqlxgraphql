#[macro_use]
extern crate sqlx;
use actix_web::{guard, web, App, HttpResponse, HttpServer};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::*;
use async_graphql_actix_web::{GQLRequest, GQLResponse};
use sqlx::postgres::{PgPoolOptions, Postgres};
use sqlx::Pool;
use std::env;

type AppQSchema = Schema<QueryUsers, EmptyMutation, EmptySubscription>;

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
        let pool = ctx.data::<web::Data<Pool<Postgres>>>()?;

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

async fn index(
    schema: web::Data<AppQSchema>,
    pool: web::Data<Pool<Postgres>>,
    req: GQLRequest,
) -> GQLResponse {
    req.into_inner().data(pool).execute(&schema).await.into()
}

async fn gql_playgound() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[actix_rt::main]
async fn main() -> std::result::Result<(), String> {
    let db_url = env::var("DATABASE_URL").expect("unable to find env variable DATABASE_URL");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| format!("{:?}", e))?;
    let schema = Schema::build(QueryUsers, EmptyMutation, EmptySubscription).finish();

    println!("Playground: http://localhost:8000");

    HttpServer::new(move || {
        App::new()
            .data(schema.clone())
            .data(pool.clone())
            .service(web::resource("/").guard(guard::Post()).to(index).app_data(
                IntoQueryBuilderOpts {
                    max_num_files: Some(3),
                    ..IntoQueryBuilderOpts::default()
                },
            ))
            .service(web::resource("/").guard(guard::Get()).to(gql_playgound))
    })
    .bind("127.0.0.1:8000")
    .map_err(|e: std::io::Error| format!("{:?}", e))?
    .run()
    .await
    .map_err(|e: std::io::Error| format!("{:?}", e))?;
    Ok(())
}
