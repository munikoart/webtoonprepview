use clap::Parser;
use std::path::{Path, PathBuf};
use url::{Url, ParseError};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use horrorshow::prelude::*;
use horrorshow::helper::doctype;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use urlencoding::encode;
use regex::Regex;


#[macro_use]
extern crate horrorshow;

/// Simple program to view your webtoon images on pc and mobile browser.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory where the images are located on your computer
    #[arg(short, long)]
    folder: String,

    /// Folder name you wish to specify relative to index.html. Default value is "img" (i.e http://localhost:8080/img)
    #[arg(short, long, default_value = "img")]
    dest: String,

    /// Build the html page
    #[arg(short, long, default_value_t = false)]
    build: bool,

    /// Spin up a simple http server to show the html page in a browser
    #[arg(short, long, default_value_t = false)]
    serve: bool,

    /// Build the html page and spin up a simple http server to view the page in a browser
    #[arg(short, long, default_value_t = false)]
    all: bool,

    /// Specify the http port to serve. Default is port 8080 (i.e. http://localhost:8080)
    #[arg(short, long, default_value = "8080")]
    port: String,
}

struct FileData {
    path: PathBuf,
    order: usize,
}


fn create_simple_html(files: &HashMap<String, FileData>, subpath: &String) {
    // Sort hashmap keys
    let my_title = "webtoon viewer";

    let mut keys: Vec<(&String, &FileData)> = files.iter().collect();
    // sort your files
    keys.sort_by(|a, b| b.1.order.cmp(&a.1.order));
    keys.reverse();

    let actual = format!("{}", html! {
        : doctype::HTML;
        html {
            head {
                // Use a variable
                title : my_title;

                link(rel="preconnect",href="https://fonts.googleapis.com"){}
                link(rel="preconnect",href="https://fonts.gstatic.com",crossorigin){}
                link(href="https://fonts.googleapis.com/css2?family=Permanent+Marker&display=swap", rel="stylesheet"){}
                link(href="https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100;0,300;0,400;0,500;0,700;0,900;1,100;1,300;1,400;1,500;1,700;1,900&display=swap",rel="stylesheet"){}
            }
            body {
                
                // attributes
                div(id="maindiv", style="display:grid; place-items:center; background-color:#eee; font-size:0") {
                    div(id="header", style="width:100%;font-size:20pt;background-color:#000;text-align:center; font-family: Permanent Marker, cursive;") {
                        h1(id="heading", class="title", style="color:white") : my_title;
                    }
                    
                    div(id="innerdiv",  style="display:block; justify-align:center; width:800px; background-color:#eee;") {

                        @for key in keys.iter() {
                           // div(id = "dev", style="display:block;padding-top:0px;padding-bottom:0px"){
                            img (src=format!("{}/{}", subpath, key.0), style="margin:0px;border:0px;height: 100%;width: 100%; object-fit: contain") {}
                            // div (style="font-size:12pt; height: 150px; font-family: Roboto; word-wrap: break-word"){
                            //      p (style="padding: 5px; background-color:#bbb;") {
                            //     // Insert raw text (unescaped)
                            //     : Raw(key.0);
                            //     }
                            // }
                            //}
                        }
                    }
                    
                    
                }  
                br;
                script {
                    : "if(/Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent)){var e1 = document.getElementById('innerdiv');e1.style.width = '100%';}else{var e1 = document.getElementById('innerdiv');e1.style.width = '800px';}"
                }
            }
        }
    });
    let mut file = File::create("index.html").unwrap();
    file.write_all(actual.as_bytes()).unwrap();
}


fn handle_connection(mut stream: TcpStream, subpath: &String, files: &HashMap<String, FileData>) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";

    if buffer.starts_with(get) {
        let contents = fs::read_to_string("index.html").unwrap();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            contents.len(),
            contents
        );
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
        
    } else {
        // some other request
        for key in files.keys() {
            let file_name = format!("GET {}/{}", subpath, key);
            if buffer.starts_with(file_name.as_bytes()) {
                let file_pathbuf = files.get(key).unwrap();
                println!("requested file: {}", file_name);
                let file_contents = fs::read(&file_pathbuf.path).unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                    file_contents.len()
                );
                
                stream.write(response.as_bytes()).unwrap();
                stream.write(&file_contents).unwrap();
                stream.flush().unwrap();
            }
        }

    }
}

fn main() {
    let mut webtoon_images: HashMap<String, FileData> = HashMap::new();
    let args = Args::parse();

    // Check if the dir is filed
    if args.folder.is_empty() {
        panic!("Error! Provided source directory for `--source` is empty");
    }
    
    // Check if the dir path provided is valid.
    let dir_path = Path::new(&args.folder);
    if !dir_path.exists() {
        panic!("Error! Provided source directory does not exist");
    }
    if !dir_path.is_dir() {
        panic!("Error! Provided source is not a directory");
    }

    // Check if dest is valid
    let test_url = format!("http://localhost:8080/{}", args.dest);
    let parsed_url = Url::parse(&test_url);
    let url_subpath = match parsed_url {
        Ok(ref u)=>u.path().to_string(),
        Err(_)=>panic!("Error! Url path {} is invalid", args.dest),
    };
    println!("Images will be placed in {}", url_subpath);

    // Build the html page
    if args.all || args.build {
        println!("Reading from folder {} ...", dir_path.display());
        // read files in folder
        let paths = fs::read_dir(dir_path).unwrap();
        let re = Regex::new(r"[^0-9]+(?<number>\d+\d*)\.\S+").unwrap();

        for path in paths {
            if let Ok(e) = path {
                let file_path = e.path();
                if file_path.is_dir() { continue; }
                let file_name = file_path.file_name().unwrap().to_string_lossy();
                //println!("Name: {}", file_name);

                let caps = re.captures(&file_name).unwrap();
                let number = caps.name("number").unwrap().as_str();
                let order = number.parse::<usize>().unwrap();
                let key = encode(&file_name).to_string();
                println!("Name: {} order: {}", key, order);
                webtoon_images.insert(key, FileData {path: file_path, order: order});
            }
        }

        create_simple_html(&webtoon_images, &url_subpath);

    }

    if args.all || args.serve {
        const HOST : &str ="0.0.0.0";

        /* Concating Host address and Port to Create Final Endpoint */
        let end_point : String = HOST.to_owned() + ":" +  &args.port;

        /*Creating TCP Listener at our end point */
        let listener = TcpListener::bind(end_point).unwrap();

        println!("Web server is listening at port {}", args.port);

        /* Conneting to any incoming connections */
        for stream in listener.incoming() {
            let _stream = stream.unwrap();
            // Call Function to process any incomming connections
            handle_connection(_stream, &url_subpath, &webtoon_images);
        }
    }
}