use axum::{routing::post, Json, Router};
use opener::OpenError;
use serde::Deserialize;
use winprint::ticket::FeatureOptionPackWithPredefined;
use winprint::ticket::PredefinedMediaName;
use winprint::ticket::PredefinedPageOutputColor;
use winprint::ticket::PrintTicketBuilder;

pub fn router() -> Router {
    Router::new()
        .route("/open", post(open_handler))
        .route("/print", post(print_handler))
}

#[derive(Deserialize)]
struct OpenFolder {
    mode: String,
    path: String,
}

async fn open_handler(Json(OpenFolder { mode, path }): Json<OpenFolder>) -> Json<String> {
    match mode.as_str() {
        "open" => match opener::open(path) {
            Ok(_) => Json("OK".into()),
            Err(e) => {
                println!("Failed to open folder: {}", e);

                match e {
                    OpenError::Io(e) => {
                        println!("IO error: {}", e);
                        Json("IO Error".into())
                    }
                    _ => Json(e.to_string()),
                }
            }
        },
        "reveal" => match opener::reveal(path) {
            Ok(_) => Json("OK".into()),
            Err(e) => Json(e.to_string()),
        },
        _ => Json("Unknown mode".into()),
    }
}

#[derive(Deserialize)]
struct PrintFolder {
    path: String,
}

async fn print_handler(Json(PrintFolder { path }): Json<PrintFolder>) -> Json<String> {
    print_file(&path);

    Json("OK".into())
}

use std::path::Path;
use winprint::printer::FilePrinter;
use winprint::printer::PdfiumPrinter;
use winprint::printer::PrinterDevice;
use winprint::ticket::FeatureOptionPack;
use winprint::ticket::PrintCapabilities;

use crate::routes::capabilities::page_scaling::{PageScaling, PredefinedPageScaling};

fn print_file(path: &str) {
    let printers = PrinterDevice::all().expect("Failed to get printers");
    let my_device = printers
        .into_iter()
        .find(|x| x.name().contains("HP PageWide Color MFP E77650"))
        .expect("My Printer not found");

    let capabilities = PrintCapabilities::fetch(&my_device).unwrap();
    // println!("Capabilities: {:#?}", capabilities); // capabilities

    let a4_media = capabilities
        .page_media_sizes()
        .find(|x| x.as_predefined_name() == Some(PredefinedMediaName::ISOA4))
        .unwrap();

    let monochrome_media = capabilities
        .page_output_colors()
        .find(|x| x.as_predefined_name() == Some(PredefinedPageOutputColor::Grayscale))
        .unwrap();

    let page_scaling = PageScaling::list(&capabilities)
        .find(|x| x.as_predefined_name() == Some(PredefinedPageScaling::Fill))
        .unwrap();

    let mut builder = PrintTicketBuilder::new(&my_device).unwrap();
    builder.merge(a4_media).unwrap();
    builder.merge(monochrome_media).unwrap();
    builder.merge(page_scaling).unwrap();
    let ticket = builder.build().unwrap();
    println!(
        "{}",
        String::from_utf8(ticket.get_xml().to_vec()).unwrap_or("NOTHING TO SEE".into())
    );

    let pdf = PdfiumPrinter::new(my_device);
    let path = Path::new(path);
    pdf.print(path, ticket).unwrap();
}
