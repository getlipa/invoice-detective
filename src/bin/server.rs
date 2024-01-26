use rocket::{get, launch, routes};
use rocket_dyn_templates::serde::Serialize;
use rocket_dyn_templates::{context, Template};

#[derive(Serialize)]
struct Node {
    id: String,
    alias: String,
}

#[get("/<invoice>")]
fn index(invoice: &str) -> Template {
    let recipient = "Non-custodial Consumer wallet";
    let mempool_space_base_url = "https://mempool.space/lightning/node";
    let node = Node {
        id: "03864ef025fde8fb587d989186ce6a4a186895ee44a926bfc370e2c366597a3f8f".to_string(),
        alias: "ACINQ".to_string(),
    };
    Template::render(
        "index",
        context! { invoice, recipient, mempool_space_base_url, node },
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .attach(Template::fairing())
}
