use serde_json::json;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| {
            Response::from_html(
                "<html><body>
                    <h2>Hello from Workers!</h2>
                    <img src='favicon.ico' />
                 </body></html>",
            )
        })
        .get_async("/favicon.ico", |_, ctx| async move {
            let kv_assets = match ctx.kv("__STATIC_CONTENT") {
                Ok(x) => x,
                Err(e) => return Response::error(format!("No site was configured: {e}"), 400),
            };
            let key = match ctx.env.asset_key("favicon.ico") {
                Ok(key) => key,
                Err(e) => return Response::error(format!("Asset key not found: {e}"), 500),
            };
            let bytes = match kv_assets.get(&key).bytes().await {
                Ok(bytes) => bytes.unwrap_or_default(),
                Err(e) => return Response::error(format!("Asset not found in KV: {e}"), 404),
            };
            let mut res = Response::from_bytes(bytes).unwrap();
            res.headers_mut().set("content-type", "image/x-icon")?;
            Ok(res)
        })
        .post_async("/form/:field", |mut req, ctx| async move {
            if let Some(name) = ctx.param("field") {
                let form = req.form_data().await?;
                match form.get(name) {
                    Some(FormEntry::Field(value)) => {
                        return Response::from_json(&json!({ name: value }))
                    }
                    Some(FormEntry::File(_)) => {
                        return Response::error("`field` param in form shouldn't be a File", 422);
                    }
                    None => return Response::error("Bad Request", 400),
                }
            }

            Response::error("Bad Request", 400)
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}
