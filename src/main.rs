use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::Nvml;
use sysinfo::{CpuExt, DiskExt};
use sysinfo::{ProcessExt, System, SystemExt};

use axum::{http::Method, routing::get, Router};
use axum_extra::routing::SpaRouter;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{net::SocketAddr, sync::Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tower_http::cors::{Any, CorsLayer};

use inquire::Select;
use std::str::FromStr;

#[macro_use]
extern crate lazy_static;

#[tokio::main]
async fn main() {
    let mut ipv4_interfaces_list: HashMap<String, String> = HashMap::new();

    for i in NetworkInterface::show().unwrap().iter() {
        if i.addr.unwrap().ip().is_ipv4() == true {
            ipv4_interfaces_list.insert(
                format!("{} <- {}", i.addr.unwrap().ip().to_string(), i.name),
                i.addr.unwrap().ip().to_string(),
            );
        }
    }

    let selected_address_select = Select::new(
        "Choose an address to bind this program to:",
        ipv4_interfaces_list
            .keys()
            .cloned()
            .collect::<Vec<String>>(),
    )
    .prompt();

    let mut ip: String;
    match selected_address_select {
        Ok(selection) => {
            ip = ipv4_interfaces_list
                .get(&selection)
                .expect("something as gone very wrong")
                .clone()
        }
        Err(_) => panic!("didnt pick one? idk something happened"),
    }

    tokio::spawn(api_server(ip.clone()));

    let addr = format!("{}:7887", ip);

    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    println!("Socket listening on: {}", addr);

    while let Ok((stream, sock)) = listener.accept().await {
        let peer = stream.peer_addr().unwrap();
        println!(
            "New socket connection on: {}, Peer: {}",
            sock.to_string(),
            peer.to_string()
        );
        tokio::spawn(handle_connection(peer, stream));
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) {
    println!("New thread spawned: Peer = {}", peer.to_string());
    let ws_stream = accept_async(stream).await.expect("Accept fail");

    let (mut _ws_sender, mut ws_reciever) = ws_stream.split();

    while let Some(response) = ws_reciever.next().await {
        let message = response.expect("error unwrapping").to_string();
        let mut state = TEMPERATURE.lock().unwrap();
        *state = String::from(message);
    }
}

async fn api_server(ip: String) {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let app = Router::new()
        .merge(SpaRouter::new("/", "assets"))
        .route("/data", get(data_route))
        .route("/data/temp", get(temperature_route))
        .layer(cors);

    let addr = format!("{}:7667", ip).parse::<SocketAddr>().unwrap();

    println!("Axum is listining:");
    println!("{}/  <- Website", addr.to_string());
    println!("{}/data  <- Data", addr.to_string());
    println!("{}/data/temp  <- CPU Temperature", addr.to_string()); //Assuming the java temperature websocket client is connected, this should be updated

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

//Axum routes

async fn data_route() -> String {
    SYSTEM_STATE.lock().unwrap().refresh_all();
    let system = &SYSTEM_STATE.lock().unwrap();
    let nvml = &NVML_STATE.lock().unwrap();
    let device = nvml.device_by_index(0).unwrap();

    let mut cpu_vec: Vec<CpuInfo> = vec![];

    for cpu in system.cpus() {
        cpu_vec.push(CpuInfo {
            brand: cpu.brand().to_string(),
            usage: cpu.cpu_usage(),
            frequency: cpu.frequency(),
        })
    }

    let mut disk_vec: Vec<DiskInfo> = vec![];

    for disk in system.disks() {
        disk_vec.push(DiskInfo {
            mount_point: String::from_str(disk.mount_point().to_str().unwrap())
                .unwrap_or("error".to_string()),
            name: String::from_str(disk.name().to_str().unwrap()).unwrap_or("error".to_string()),
            size: disk.total_space(),
            used_space: disk.total_space() - disk.available_space(),
        })
    }

    format!(
        "{}",
        serde_json::to_string(&CurrentData {
            system_details: SystemInfo {
                name: (system.name().unwrap_or("error".to_string())),
                kernel_ver: (system.kernel_version().unwrap_or("error".to_string())),
                os_ver: (system.os_version().unwrap_or("error".to_string())),
                host_name: (system.host_name().unwrap_or("error".to_string())),
                uptime: (system.uptime()),
            },
            cpu: cpu_vec,
            ram: RamInfo {
                total_memory: (system.total_memory()),
                used_memory: (system.used_memory()),
                total_swap: (system.total_swap()),
                used_swap: (system.used_swap()),
            },
            disk: disk_vec,
            gpu: GpuInfo {
                name: (device.name().unwrap_or("error".to_string())),
                util: (device.utilization_rates().expect("error").gpu),
                encoder_util: (device.encoder_utilization().expect("error").utilization),
                decoder_util: (device.decoder_utilization().expect("error").utilization),
                used_memory: (device.memory_info().expect("error").used),
                total_memory: (device.memory_info().expect("error").total),
                graphic_clock: (device.clock_info(Clock::Graphics).unwrap_or(101)),
                max_graphic_clock: (device.max_clock_info(Clock::Graphics).unwrap_or(101)),
                memory_clock: (device.clock_info(Clock::Memory).unwrap_or(101)),
                memory_clock_max: (device.clock_info(Clock::Memory).unwrap_or(101)),
                sm_clock: (device.clock_info(Clock::SM).unwrap_or(101)),
                sm_clock_max: (device.max_clock_info(Clock::SM).unwrap_or(101)),
                video_clock: (device.clock_info(Clock::Video).unwrap_or(101)),
                video_clock_max: (device.max_clock_info(Clock::Video).unwrap_or(101)),
                power: (device.power_usage().unwrap_or(1000) / 1000),
                max_power: (device.power_management_limit_default().unwrap_or(1000) / 1000),
                temperature: (device.temperature(TemperatureSensor::Gpu).unwrap_or(0)),
            },
        })
        .unwrap()
    )
}

async fn temperature_route() -> String {
    let data = format!("{}", TEMPERATURE.lock().unwrap());
    return data;
}

#[derive(Serialize, Deserialize)]
struct CurrentData {
    system_details: SystemInfo,
    cpu: Vec<CpuInfo>,
    ram: RamInfo,
    disk: Vec<DiskInfo>,
    gpu: GpuInfo,
}

#[derive(Serialize, Deserialize)]
struct SystemInfo {
    name: String,
    kernel_ver: String,
    os_ver: String,
    host_name: String,
    uptime: u64,
}

#[derive(Serialize, Deserialize)]
struct CpuInfo {
    brand: String,
    usage: f32,
    frequency: u64,
}

#[derive(Serialize, Deserialize)]
struct RamInfo {
    total_memory: u64,
    used_memory: u64,
    total_swap: u64,
    used_swap: u64,
}

#[derive(Serialize, Deserialize)]
struct DiskInfo {
    mount_point: String,
    name: String,
    size: u64,
    used_space: u64,
}

#[derive(Serialize, Deserialize)]
struct GpuInfo {
    name: String,
    util: u32,
    encoder_util: u32,
    decoder_util: u32,
    used_memory: u64,
    total_memory: u64,
    graphic_clock: u32,
    max_graphic_clock: u32,
    memory_clock: u32,
    memory_clock_max: u32,
    sm_clock: u32,
    sm_clock_max: u32,
    video_clock: u32,
    video_clock_max: u32,
    power: u32,
    max_power: u32,
    temperature: u32,
}

lazy_static! {
    static ref TEMPERATURE: Mutex<String> = Mutex::new(String::from("-1"));
    static ref SYSTEM_STATE: Mutex<System> = Mutex::new(System::new_all());
    static ref NVML_STATE: Mutex<Nvml> = Mutex::new(Nvml::init().unwrap());
}
