use crate::message::{AddResponse, server_message, ClientMessage, client_message, ServerMessage};
use log::{error, info, warn};
use prost::Message;
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

// Represents a connected client
struct Client {
    stream: TcpStream, // The TCP connection for the client
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        stream.set_nonblocking(true).unwrap(); // Set the TCP stream to non-blocking mode
        Client { stream }
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let mut buffer = [0; 512]; // Buffer to hold incoming data

        loop {
            let bytes_read = match self.stream.read(&mut buffer) {
                Ok(bytes) => bytes, // Successfully read some bytes
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10)); // Wait briefly before retrying
                    continue;
                }
                Err(e) => {
                    return Err(e); // Return on other errors
                }
            };

            if bytes_read == 0 {
                info!("Client disconnected.");
                return Ok(()); // Connection closed by the client
            }

            match ClientMessage::decode(&buffer[..bytes_read]) {
                Ok(client_message) => match client_message.message {
                    //in case of echo message
                    Some(client_message::Message::EchoMessage(echo_message)) => {
                        info!("Received EchoMessage: {}", echo_message.content);

                        let payload = ServerMessage {
                            message: Some(server_message::Message::EchoMessage(echo_message)),
                        }
                        .encode_to_vec();

                        self.stream.write_all(&payload)?; // Send back the echoed message
                        self.stream.flush()?; // Ensure the message is sent immediately
                    }
                    //in case of add request message
                    Some(client_message::Message::AddRequest(add_request)) => {
                        info!("Received AddRequest: {} + {}", add_request.a, add_request.b);

                        let result = add_request.a + add_request.b; // Perform addition
                        let add_response = AddResponse { result };

                        let payload = ServerMessage {
                            message: Some(server_message::Message::AddResponse(add_response)),
                        }
                        .encode_to_vec();

                        self.stream.write_all(&payload)?; // Send the addition result
                        self.stream.flush()?; // Ensure the response is sent
                    }
                    None => {
                        error!("Received a ClientMessage with no message!");
                    }
                },
                Err(e) => {
                    error!("Failed to decode ClientMessage: {}", e); // Log decoding errors
                }
            }
        }
    }
}

pub struct Server {
    listener: TcpListener, // Listener for incoming connections
    is_running: Arc<AtomicBool>, // Shared flag to control server status
    client_threads: Arc<Mutex<Vec<thread::JoinHandle<()>>>>, // Threads handling clients
}

impl Server {
    pub fn new(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?; // Bind the listener to the address
        let is_running = Arc::new(AtomicBool::new(false)); // Initialize running state
        let client_threads = Arc::new(Mutex::new(Vec::new())); // Initialize thread storage

        Ok(Server {
            listener,
            is_running,
            client_threads,
        })
    }

    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst); // Set running flag to false
            info!("Shutdown signal sent.");

            let mut threads = self.client_threads.lock().unwrap(); // Lock threads list(shared resource)
            for handle in threads.drain(..) {
                //join all threads 
                if let Err(e) = handle.join() {
                    error!("Failed to join thread: {:?}", e); // Log thread join errors
                }
            }
            info!("All client threads joined.");
        } else {
            warn!("Server was already stopped or not running.");
        }
    }

    pub fn run(&self) -> io::Result<()> {
        self.is_running.store(true, Ordering::SeqCst); // Set running flag to true
        info!("Server is running on {}", self.listener.local_addr()?); // Log server address

        self.listener.set_nonblocking(true)?; // Set listener to non-blocking mode

        while self.is_running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("New client connected: {}", addr); // Log new client connection

                    let is_running = Arc::clone(&self.is_running); // Clone running flag
                    let client_threads = Arc::clone(&self.client_threads); // Clone threads list
                    //creating thread for new client
                    let handle = thread::spawn(move || {
                        let mut client = Client::new(stream); // Initialize client handler
                        while is_running.load(Ordering::SeqCst) {
                            if let Err(e) = client.handle() {
                                error!("Error handling client: {}", e); // Log client errors
                                break;
                            }
                        }

                        if let Err(e) = client.stream.shutdown(std::net::Shutdown::Both) {
                            error!("Failed to shutdown stream: {}", e); // Log shutdown errors
                        }
                    });

                    client_threads.lock().unwrap().push(handle); // Store thread handle so the stop can join each thread
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10)); // Wait before retrying
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e); // Log accept errors
                }
            }
        }

        info!("Server stopped."); // Log server stop
        Ok(())
    }
}
