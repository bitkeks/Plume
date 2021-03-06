use rocket::{
    http::{Cookie, Cookies, uri::Uri},
    response::Redirect,
    request::{LenientForm,FlashMessage}
};
use rocket_contrib::Template;
use std::borrow::Cow;
use validator::{Validate, ValidationError, ValidationErrors};

use plume_models::{
    db_conn::DbConn,
    users::{User, AUTH_COOKIE}
};

#[get("/login")]
fn new(user: Option<User>) -> Template {
    Template::render("session/login", json!({
        "account": user,
        "errors": null, 
        "form": null
    }))
}

#[derive(FromForm)]
struct Message {
	m: String
}

#[get("/login?<message>")]
fn new_message(user: Option<User>, message: Message) -> Template {
    Template::render("session/login", json!({
        "account": user,
        "message": message.m,
        "errors": null,
        "form": null
    }))
}


#[derive(FromForm, Validate, Serialize)]
struct LoginForm {
    #[validate(length(min = "1", message = "We need an email or a username to identify you"))]
    email_or_name: String,
    #[validate(length(min = "1", message = "Your password can't be empty"))]
    password: String
}

#[post("/login", data = "<data>")]
fn create(conn: DbConn, data: LenientForm<LoginForm>, flash: Option<FlashMessage>, mut cookies: Cookies) -> Result<Redirect, Template> {
    let form = data.get();
    let user = User::find_by_email(&*conn, form.email_or_name.to_string())
        .map(|u| Ok(u))
        .unwrap_or_else(|| User::find_local(&*conn, form.email_or_name.to_string()).map(|u| Ok(u)).unwrap_or(Err(())));

    let mut errors = match form.validate() {
        Ok(_) => ValidationErrors::new(),
        Err(e) => e
    };
    if let Err(_) = user.clone() {
        let mut err = ValidationError::new("invalid_login");
        err.message = Some(Cow::from("Invalid username or password"));
        errors.add("email_or_name", err)
    } else if !user.clone().expect("User not found").auth(form.password.clone()) {
        let mut err = ValidationError::new("invalid_login");
        err.message = Some(Cow::from("Invalid username or password"));
        errors.add("email_or_name", err)
    }

    if errors.is_empty() {
        cookies.add_private(Cookie::new(AUTH_COOKIE, user.unwrap().id.to_string()));
        Ok(Redirect::to(Uri::new(flash
            .and_then(|f| if f.name() == "callback" { Some(f.msg().to_owned()) } else { None })
            .unwrap_or("/".to_owned()))
        ))
    } else {
        println!("{:?}", errors);
        Err(Template::render("session/login", json!({
            "account": null,
            "errors": errors.inner(),
            "form": form
        })))
    }
}

#[get("/logout")]
fn delete(mut cookies: Cookies) -> Redirect {
    let cookie = cookies.get_private(AUTH_COOKIE).unwrap();
    cookies.remove_private(cookie);
    Redirect::to("/")
}
