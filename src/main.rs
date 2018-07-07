#![feature(custom_derive, decl_macro, plugin)]
#![plugin(rocket_codegen)]

extern crate activitypub;
extern crate colored;
extern crate diesel;
extern crate dotenv;
extern crate failure;
extern crate gettextrs;
extern crate heck;
extern crate plume_common;
extern crate plume_models;
extern crate rocket;
extern crate rocket_contrib;
extern crate rocket_csrf;
extern crate rocket_i18n;
extern crate rpassword;
#[macro_use]
extern crate serde_json;
extern crate webfinger;

use rocket_contrib::Template;
use rocket_contrib::tera::{GlobalFn, from_value, to_value, Error};
use rocket_csrf::CsrfFairingBuilder;
use std::collections::HashMap;

mod inbox;
mod setup;
mod routes;

/// Global url_for function used in Tera templates.
/// Renders Rocket route endpoint names to their respective URI.
/// Note: Tera escapes HTML content received from global functions, so `/` is escaped.
/// Note: **Duplicate route function names are overwritten.**
fn make_url_for(routes_map: HashMap<String, String>) -> GlobalFn {
    // Teras GlobalFn expects a Boxed closure which returns a Result<Value>.
    // TODO: figure out how to handle URI segments (variables in route paths)
    Box::new(move |args: HashMap<String, serde_json::Value>| -> Result<serde_json::Value, Error> {
        match args.get("name") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) => Ok(to_value(routes_map.get(&v).unwrap_or(&"/".to_string())).unwrap()),
                Err(_) => Err("name key/value could not be parsed!".into()),
            },
            None => Err("key 'name' does not exist".into()),
        }
    })
}

fn main() {
    let pool = setup::check();
    rocket::ignite()
        .mount("/", routes![
            routes::blogs::details,
            routes::blogs::activity_details,
            routes::blogs::outbox,
            routes::blogs::new,
            routes::blogs::new_auth,
            routes::blogs::create,

            routes::comments::create,

            routes::instance::index,
            routes::instance::shared_inbox,
            routes::instance::nodeinfo,

            routes::likes::create,
            routes::likes::create_auth,

            routes::notifications::notifications,
            routes::notifications::notifications_auth,

            routes::posts::details,
            routes::posts::details_response,
            routes::posts::activity_details,
            routes::posts::new,
            routes::posts::new_auth,
            routes::posts::create,

            routes::reshares::create,
            routes::reshares::create_auth,

            routes::session::new,
            routes::session::new_message,
            routes::session::create,
            routes::session::delete,

            routes::static_files,

            routes::user::me,
            routes::user::details,
            routes::user::dashboard,
            routes::user::dashboard_auth,
            routes::user::followers,
            routes::user::edit,
            routes::user::edit_auth,
            routes::user::update,
            routes::user::follow,
            routes::user::follow_auth,
            routes::user::activity_details,
            routes::user::outbox,
            routes::user::inbox,
            routes::user::ap_followers,
            routes::user::new,
            routes::user::create,

            routes::well_known::host_meta,
            routes::well_known::nodeinfo,
            routes::well_known::webfinger,

            routes::errors::csrf_violation
        ])
        .catch(catchers![
            routes::errors::not_found,
            routes::errors::server_error
        ])
        .manage(pool)
        .attach(rocket::fairing::AdHoc::on_attach(|rocket| {
            // Create a HashMap of routes with their name and URI and pass the url_for
            // function with this map to Tera. The function can then be called in Tera
            // templates to render route URIs from their function name.
            // This Fairing is executed as soon as `.attach` is called on it.
            let mut routes_map = HashMap::new();
            for route in rocket.routes() {
                match route.name {
                    Some(name) => { routes_map.insert(name.to_string(), route.uri.to_string()); },
                    None => { println!("No name for route URI {}", route.uri) }
                }
            }

            // Now that we have the map, attach the Template customization function,
            // apply rocket_i18n and register the global function.
            // Returns the modified Rocket instance which must be wrapped into Result below.
            let rocket = rocket.attach(Template::custom(move |engines| {
                rocket_i18n::tera(&mut engines.tera);
                engines.tera.register_global_function("url_for", make_url_for(routes_map.clone()));
            }));

            Ok(rocket)
        }))
        .attach(rocket_i18n::I18n::new("plume"))
        .attach(CsrfFairingBuilder::new()
                .set_default_target("/csrf-violation?target=<uri>".to_owned(), rocket::http::Method::Post)
                .add_exceptions(vec![
                    ("/inbox".to_owned(), "/inbox".to_owned(), rocket::http::Method::Post),
                    ("/@/<name>/inbox".to_owned(), "/@/<name>/inbox".to_owned(), rocket::http::Method::Post),

                ])
                .finalize().unwrap())

        .launch();
}
