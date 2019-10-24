#![allow(unused_variables, unused_imports, dead_code)]

#[macro_use]
extern crate diesel;

use juniper::{Executor, FieldResult};
use juniper_eager_loading::{prelude::*, EagerLoading, HasMany};
use juniper_from_schema::graphql_schema;
use std::error::Error;

// the examples all use Diesel, but this library is data store agnostic
use diesel::prelude::*;

graphql_schema! {
    schema {
      query: Query
    }

    type Query {
      countries: [Country!]! @juniper(ownership: "owned")
    }

    type User {
        id: Int!
    }

    type Country {
        id: Int!
        users: [User!]!
    }
}

mod db_schema {
    table! {
        users {
            id -> Integer,
            country_id -> Integer,
        }
    }

    table! {
        countries {
            id -> Integer,
        }
    }
}

mod models {
    use diesel::prelude::*;

    #[derive(Clone, Debug, Queryable)]
    pub struct User {
        pub id: i32,
        pub country_id: i32,
    }

    #[derive(Clone, Debug, Queryable)]
    pub struct Country {
        pub id: i32,
    }

    impl juniper_eager_loading::LoadFrom<Country> for User {
        type Error = diesel::result::Error;
        type Connection = PgConnection;

        fn load(
            countries: &[Country],
            _field_args: &(),
            db: &Self::Connection,
        ) -> Result<Vec<Self>, Self::Error> {
            use crate::db_schema::users::dsl::*;
            use diesel::pg::expression::dsl::any;

            let country_ids = countries
                .iter()
                .map(|country| country.id)
                .collect::<Vec<_>>();

            users
                .filter(country_id.eq(any(country_ids)))
                .load::<User>(db)
        }
    }
}

pub struct Query;

impl QueryFields for Query {
    fn field_countries(
        &self,
        executor: &Executor<'_, Context>,
        trail: &QueryTrail<'_, Country, Walked>,
    ) -> FieldResult<Vec<Country>> {
        let db = &executor.context().db;
        let country_models = db_schema::countries::table.load::<models::Country>(db)?;
        let mut country = Country::from_db_models(&country_models);
        Country::eager_load_all_children_for_each(&mut country, &country_models, db, trail)?;

        Ok(country)
    }
}

pub struct Context {
    db: PgConnection,
}

impl juniper::Context for Context {}

#[derive(Clone, EagerLoading)]
#[eager_loading(connection = "PgConnection", error = "diesel::result::Error")]
pub struct User {
    user: models::User,
}

impl UserFields for User {
    fn field_id(&self, executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.user.id)
    }
}

#[derive(Clone, EagerLoading)]
#[eager_loading(connection = "PgConnection", error = "diesel::result::Error")]
pub struct Country {
    country: models::Country,

    #[has_many(root_model_field = "user")]
    users: HasMany<User>,
}

impl CountryFields for Country {
    fn field_id(&self, executor: &Executor<'_, Context>) -> FieldResult<&i32> {
        Ok(&self.country.id)
    }

    fn field_users(
        &self,
        executor: &Executor<'_, Context>,
        trail: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<&Vec<User>> {
        self.users.try_unwrap().map_err(From::from)
    }
}

fn main() {}