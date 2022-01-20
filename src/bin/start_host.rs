use rhiza::host::{Host, HostConfig};
// use tokio::time::{sleep, Duration};
use tokio::signal;

use clap::{App, Arg};
use tracing_appender;
use tracing_subscriber;

fn main() -> ! {
    let file_appender = tracing_appender::rolling::minutely("logs/", "start_host");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    let matches = App::new("Rhiza Host")
        .version("0.1")
        .author("Christopher Moran <christopher.and.moran@gmail.com>")
        .about("Start a Rhiza host")
        .arg(
            Arg::new("interface")
                .short('i')
                .long("interface")
                .default_value("lo")
                .help("Sets the proper network interface"),
        )
        .arg(
            Arg::new("socket")
                .short('s')
                .long("tcp_socket_num")
                .default_value("25000")
                .help("Sets an alternative TCP socket"),
        )
        .arg(
            Arg::new("store_filename")
                .short('f')
                .long("store_filename")
                .default_value("store")
                .help("Sets the filename for the `sled`-based key-value store"),
        )
        .get_matches();

    let interface: String = matches.value_of("interface").unwrap().to_string();
    let store_filename: String = matches.value_of("store_filename").unwrap().to_string();
    let socket: usize = matches.value_of("socket").unwrap().parse().unwrap();

    let mut host: Host = HostConfig::new(interface)
        .socket_num(socket)
        .store_filename(store_filename)
        .build()
        .unwrap();
    host.start().unwrap();

    println!("Rhiza Host should be running");
    // Other tasks can operate while the host is running on it's own thread
    loop {}
}
