//! Minimal HTTP server that provides update images for [SWUpdate](https://sbabic.github.io/swupdate/swupdate.html)
//! clients.
//!
//! The server listens on a given IP address and port and provides a root endpoint `/` that can be used by SWUpdate
//! clients to check for available updates and serves the update images then on the endpoint `/images`.
//!
//! The server is implemented using the [Actix web framework](https://actix.rs/).
//!
//! # Update Request
//!
//! The server expects the following parameters in the query string of the request to `/`:
//! - `image`: Identifier of the image (to distinguish different images / applications).
//! - `device`: Identifier of the hardware / device type.
//! - `current_version`: The currently installed version of the image. An update image is only provided by the server
//!    if the version of the image on the server is different to the given version.
//!
//! ## Example
//!
//! A request could look like this:
//!
//! ```shell
//! curl -v "http://localhost:8080/?image=test-app&device=raspberrypi0-2w-64&current_version=0.1.0"
//! ```
//!
//! ## Response
//!
//! The server might respond with any of the following HTTP status codes:
//! - *302 (found)*: An update is available. The response contains a `Location` header that points to the update image.
//! - *400 (bad request)*: The request is missing one or more of the parameters.
//! - *404 (not found)*: No update is available.
//! - *500 (internal server error)*: The server could not successfully process the request. E.g., there was more than
//!   one matching update image placed in the images directory.
//!
//! ## Processing in the Server
//!
//! The server reads all files from the images directory (given as a command line parameter at server starting) and
//! filters them based on the request parameters. It expects that either one or zero update images will match the
//! filter criteria.
//!
//! # Image Serving
//!
//! The update images are served on the `/images` endpoint. The server reads all files from a given directory and
//! serves them under this path. The images are served as static files and can be downloaded by the clients.
//! Clients would usually use the `Location` header from the response to the update request to download the update
//! image.
//!
//! To provide an update image, the file just needs to be placed in the images directory. There need to be a few rules
//! followed, however, to make the server work correctly:
//! - The filename of the update image needs to contain the following fields separated by the given separator (which by
//!   default is `_`):
//!   - The image identifier
//!   - The device type
//!   - The version number
//!  - There must always be only one update image available for a given image identifier and device type. If there are
//!    more than one, the server will respond with a 500 (internal server error) status code.
//!
//! # Starting and Configuring the Server
//!
//! The minimal command line for starting the server is as follows:
//!
//! ```shell
//! swupdate-httpd --images_directory /path/to/update-images
//! ```
//! In addition, the following optional parameters can be provided:
//! - `listen_ip`: The interface to listen on.
//! - `listen_port`: The port to listen on.
//! - `filename_fields_separator`: The separator used in the filename to separate the fields.
//! - `filename_field_image_identifier`: The index of the field in the filename that contains the image identifier.
//! - `filename_field_device_type`: The index of the field in the filename that contains the device type.
//! - `filename_field_version`: The index of the field in the filename that contains the version number.
//!
use actix_files;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::Parser;
use serde_derive::Deserialize;
use std::fs;

/// Command line arguments of the application.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The directory where the update images are placed.
    #[arg(long)]
    images_directory: String,

    /// The interface to listen on.
    #[arg(long, default_value = "0.0.0.0")]
    listen_ip: String,

    /// The port to listen on.
    #[arg(long, default_value = "8080")]
    listen_port: u16,

    /// The separator used in the filename to separate the fields.
    #[arg(long, default_value = "_")]
    filename_fields_separator: String,

    /// The index of the field in the filename that contains the image identifier.
    #[arg(long, default_value = "0")]
    filename_field_image_identifier: usize,

    /// The index of the field in the filename that contains the device type.
    #[arg(long, default_value = "1")]
    filename_field_device_type: usize,

    /// The index of the field in the filename that contains the version number.
    #[arg(long, default_value = "2")]
    filename_field_version: usize,
}

/// Data that needs to be available to the request handlers.
struct AppData {
    images_directory: String,
    filename_fields_separator: String,
    filename_field_image_identifier: usize,
    filename_field_device_type: usize,
    filename_field_version: usize,
}

/// Parameters of the HTTP request to the update endpoint.
///
/// All of them need to be provided by the client, otherwise the request will fail with a 400 (bad request) status
/// code.
#[derive(Debug, Deserialize)]
pub struct UpdateRequest {
    /// Identifier of the image (to distinguish different images / applications).
    image: Option<String>,

    /// Identifier of the hardware / device type.
    device: Option<String>,

    /// The currently installed version of the image. An update image is only provided by the server if the version of
    /// the image on the server is different to the given version.
    current_version: Option<String>,
}

/// Request handler for the update endpoint.
#[get("/")]
async fn update(req: HttpRequest, info: web::Query<UpdateRequest>) -> impl Responder {
    let app_data: &AppData = req.app_data::<AppData>().unwrap();

    // check that all parameters are set and respond with a 400 (bad request) if not
    if info.image.is_none() || info.device.is_none() || info.current_version.is_none() {
        return HttpResponse::BadRequest().finish();
    }

    // read all file paths from the images directory
    let Ok(paths) = fs::read_dir(app_data.images_directory.as_str()) else {
        return HttpResponse::InternalServerError().finish();
    };

    // filter the file paths to only include the ones that match the device type
    let image_files: Vec<_> = paths
        .filter(|r| r.is_ok())
        .map(|r| {
            r.unwrap() // get the paths from the read directory result
                .file_name() // get the filenames string for the paths
                .into_string()
                .unwrap() // already tested that 'is_ok' before
        })
        .filter(|f| {
            let without_extension = f.rsplit_once('.').unwrap().0;
            let splitted: Vec<&str> = without_extension
                .split(&app_data.filename_fields_separator)
                .collect();
            splitted[app_data.filename_field_image_identifier] == info.image.as_ref().unwrap()
                && splitted[app_data.filename_field_device_type] == info.device.as_ref().unwrap()
        })
        .collect();

    // more than one matching update image available, which is an error
    if image_files.len() > 1 {
        return HttpResponse::InternalServerError()
            .insert_header(("X-Error", "More than one matching update image."))
            .finish();
    }

    // no update available if no matching file could be found or the version of the file is the same as the given version
    if image_files.is_empty()
        || image_files[0]
            .rsplit_once('.')
            .unwrap()
            .0
            .split(&app_data.filename_fields_separator)
            .collect::<Vec<&str>>()[app_data.filename_field_version]
            == info.current_version.as_ref().unwrap()
    {
        return HttpResponse::NotFound().finish();
    }

    // all parameters are set and update is available, hence we can process the request
    HttpResponse::Found()
        .insert_header(("Location", format!("/images/{}", image_files[0])))
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    HttpServer::new(move || {
        App::new()
            .app_data(AppData {
                images_directory: args.images_directory.clone(),
                filename_fields_separator: args.filename_fields_separator.clone(),
                filename_field_image_identifier: args.filename_field_image_identifier,
                filename_field_device_type: args.filename_field_device_type,
                filename_field_version: args.filename_field_version,
            })
            .service(update)
            .service(
                actix_files::Files::new("/images", args.images_directory.as_str())
                    .show_files_listing(),
            )
    })
    .bind((args.listen_ip, args.listen_port))?
    .run()
    .await
}
