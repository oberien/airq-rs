use include_dir::Dir;
use rocket::handler::{Handler, Outcome};
use rocket::http::{Method, ContentType};
use rocket::response::Responder;
use rocket::{Request, Data, Route};

#[derive(Clone)]
pub struct IncludedStaticFiles;

const DIR: Dir = include_dir::include_dir!("../static/");
#[rocket::async_trait]
impl Handler for IncludedStaticFiles {
    async fn handle<'r, 's: 'r>(&'s self, request: &'r Request<'_>, data: Data) -> Outcome<'r> {
        let search_for = match request.uri().path() {
            "/" => "/index.html",
            path => path,
        };
        assert!(search_for.starts_with("/"));
        match DIR.get_file(&search_for[1..]) {
            None => Outcome::Forward(data),
            Some(file) => {
                let response = file.contents().respond_to(request)
                    .map(|mut response| {
                        if let Some(ext) = file.path().extension() {
                            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                                response.set_header(ct);
                            }
                        }
                        response
                    });
                Outcome::try_from(request, response)
            }
        }
    }
}
impl Into<Vec<Route>> for IncludedStaticFiles {
    fn into(self) -> Vec<Route> {
        vec![
            Route::ranked(10, Method::Get, "/", self.clone()),
            Route::ranked(10, Method::Get, "/<path..>", self),
        ]
    }
}
