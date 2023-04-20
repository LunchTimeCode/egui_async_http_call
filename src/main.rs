#![deny(clippy::all)]
#![forbid(unsafe_code)]

use eframe::egui;
use eframe::epaint::ahash::{HashMap, HashMapExt};
use egui_extras::{Column, TableBuilder};
use log::info;
use reqwest::Client;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use tokio::runtime::Runtime;

struct AsyncApp {
    tx: Sender<RawResponse>,
    rx: Receiver<RawResponse>,

    incoming: RawResponse,
}

#[derive(Debug, Default)]
struct RawResponse {
    headers: HashMap<String, String>,
    body: String,
}

fn main() {
    let rt = Runtime::new().expect("Unable to create Runtime");

    let _enter = rt.enter();

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    egui_logger::init().unwrap();
    // Run the GUI in the main thread.
    let _ = eframe::run_native(
        "async egui",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::<AsyncApp>::default()),
    );
}

impl Default for AsyncApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            tx,
            rx,
            incoming: RawResponse::default(),
        }
    }
}

impl eframe::App for AsyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update the counter with the async response.
        if let Ok(response) = self.rx.try_recv() {
            self.incoming = response
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Press the button to initiate an HTTP request.");

            if ui.button("send").clicked() {
                send_req(self.tx.clone(), ctx.clone());
            }
            egui::ScrollArea::vertical()
                .id_source("some inner")
                .max_height(400.0)
                .show(ui, |ui| {
                    ui.push_id("second", |ui| {
                        egui_logger::logger_ui(ui);
                    });
                });
            ui.collapsing("raw response", |ui| {
                ui.label("headers");
                egui::ScrollArea::vertical()
                    .id_source("first")
                    .max_height(400.0)
                    .show(ui, |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .column(Column::remainder())
                            .column(Column::remainder())
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.heading("key");
                                });
                                header.col(|ui| {
                                    ui.heading("value");
                                });
                            })
                            .body(|mut body| {
                                for (key, value) in &self.incoming.headers {
                                    body.row(30.0, |mut row| {
                                        row.col(|ui| {
                                            ui.label(key);
                                        });
                                        row.col(|ui| {
                                            ui.label(value);
                                        });
                                    });
                                }
                            });

                        ui.separator();
                        ui.label("body");

                        ui.label(&self.incoming.body);
                    });
            });
        });
    }
}

fn send_req(tx: Sender<RawResponse>, ctx: egui::Context) {
    tokio::spawn(async move {
        info!("respond sending");
        // Send a request with an increment value.
        let res = Client::default()
            .post("https://httpbin.org/anything")
            .send()
            .await
            .expect("Unable to send request");

        info!("respond received");

        let mut headers: HashMap<String, String> = HashMap::new();

        for (key, value) in res.headers().into_iter() {
            headers.insert(
                key.to_string(),
                value.to_str().unwrap_or("nothing found").into(),
            );
        }

        let body = res.text().await.unwrap_or("nothing in body".into());

        let raw = RawResponse { headers, body };

        info!("raw: {:#?}", raw);

        // After parsing the response, notify the GUI thread of the increment value.
        let _ = tx.send(raw);
        ctx.request_repaint();
    });
}
