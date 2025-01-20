use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::misc::{print, sleep, window};
use nx::{NodeDataPopulated, NodeS, NodeSH, WSRequest};
use web_sys::WebSocket;

#[derive(Debug, Clone)]
pub enum WSResponse {
    Empty,
    Ok(NodeSH),
    Error(String),
    Message(String),
}

// Always expects either a 3 or 4 length path parameter
pub async fn get_data_if_missing_hashmap(
    ws: &WebSocket,
    path: &[String],
    tnh: Arc<Mutex<WSResponse>>,
    complete_hash_map: &OnceLock<RwLock<NodeSH>>,
) -> Result<(), String> {
    print(&format!("Getting file {:?}", path));
    let mut node_exists: bool = false;

    {
        let mut current_node: &NodeSH = &complete_hash_map.get().unwrap().read().unwrap();
        node_exists = match current_node.children.get(&path[0]) {
            None => false,
            Some(n) => match n.children.get(&path[1]) {
                None => false,
                Some(ns) => match ns.children.get(&path[2]) {
                    None => false,
                    Some(_) => true,
                },
            },
        }
    }

    print(&format!("Exist {:?}", node_exists));
    if !node_exists {
        get_img_file_hashmap(
            ws,
            nx::WSRequest {
                path: path.join("/").clone(),
            },
            tnh.clone(),
            complete_hash_map,
        )
        .await?;
    }
    print(&format!("Finished {:?}", path));
    Ok(())
}

pub async fn get_full_img_file(
    ws: &WebSocket,
    path: String,
    tnh: Arc<Mutex<WSResponse>>,
    complete_hash_map: &OnceLock<RwLock<NodeSH>>,
) -> Result<(), String> {
    match get_img_file_hashmap(ws, nx::WSRequest { path }, tnh.clone(), complete_hash_map).await {
        Ok(dep) => {
            print(&format!("{:?}", dep));
            for path in dep {
                get_img_file_hashmap(ws, nx::WSRequest { path }, tnh.clone(), complete_hash_map)
                    .await
                    .unwrap();
            }
            Ok(())
        }
        Err(e) => Err(format!("Error: {}", e)),
    }
}

pub async fn get_map_file_hashmap(
    ws: &WebSocket,
    map_id: &str,
    tnh: Arc<Mutex<WSResponse>>,
    complete_hash_map: &OnceLock<RwLock<NodeSH>>,
) -> Result<(), String> {
    match get_img_file_hashmap(
        ws,
        nx::WSRequest {
            path: format!(
                "Map.nx/Map/Map{}/{}.img",
                map_id.chars().next().unwrap(),
                map_id
            ),
        },
        tnh.clone(),
        complete_hash_map,
    )
    .await
    {
        Ok(dep) => {
            print(&format!("{:?}", dep));
            for path in dep {
                get_img_file_hashmap(ws, nx::WSRequest { path }, tnh.clone(), complete_hash_map)
                    .await
                    .unwrap();
            }
            Ok(())
        }
        Err(e) => Err(format!("Error: {}", e)),
    }
}

// Gets img file and returns dependencies that the IMG file asks for (in Back, Tile, and Obj)
pub async fn get_img_file_hashmap(
    ws: &WebSocket,
    p: nx::WSRequest,
    tnh: Arc<Mutex<WSResponse>>, // shorthand for tmp_node_holder
    complete_hash_map: &OnceLock<RwLock<NodeSH>>,
) -> Result<Vec<String>, String> {
    let start = window().performance().unwrap().now();
    match serde_json::to_string(&p) {
        Ok(payload) => {
            match ws.send_with_str(&payload) {
                Ok(_) => print(&format!("Request for {:?}", &p)),
                Err(e) => print(&format!("Unable to request {:?}, Err {:?}", &p, e)),
            };
        }
        Err(e) => {
            print(&format!("Unable to serialize {:?}, Err {:?}", &p, e));
            panic!("Unable to serialize")
        }
    };

    let mut is_data_ready = false;

    while !is_data_ready {
        // print(&format!("I am going to sleep to wait websocket to populate data {}", p.file.clone()));
        sleep(250).await;
        is_data_ready = {
            match tnh.try_lock() {
                Ok(s) => match *s {
                    WSResponse::Empty => false,
                    WSResponse::Ok(_) => true,
                    WSResponse::Error(_) => true,
                    WSResponse::Message(_) => true,
                },
                Err(_) => false,
            }
        };
    }

    let mut imgs_to_grab: HashSet<String> = HashSet::new();

    let mut tmp_h_m_clone = Arc::clone(&tnh);
    {
        let mut tmp_node_editor = tmp_h_m_clone.try_lock();
        match tmp_node_editor {
            Ok(s) => {
                match (*s).clone() {
                    WSResponse::Empty => {
                        panic!("Impossible scenario- empty response");
                    }
                    WSResponse::Ok(node_data) => {
                        // puts data in right spot
                        let path = p.path.split("/");
                        match complete_hash_map.get().unwrap().try_write() {
                            Ok(mut existing) => {
                                let mut current_path = &mut existing.children;
                                let mut current_node = "";
                                for (i, node) in path.clone().enumerate() {
                                    current_node = node;
                                    if !current_path.contains_key(node) {
                                        let mut tmp_hash_map: HashMap<String, nx::NodeSH> =
                                            HashMap::new();
                                        // If statement here prevents double creation of node
                                        if i != path.clone().collect::<Vec<&str>>().len() - 1 {
                                            current_path.insert(
                                                node.to_string(),
                                                nx::NodeSH {
                                                    data: nx::NodeDataPopulated::None,
                                                    children: tmp_hash_map,
                                                },
                                            );
                                        }
                                    };

                                    if i != path.clone().collect::<Vec<&str>>().len() - 1 {
                                        current_path =
                                            &mut current_path.get_mut(node).unwrap().children;
                                    }
                                }
                                current_path.insert(current_node.to_string(), node_data.clone());
                            }
                            Err(e) => {
                                panic!("Cannot obtain writer for complete_hash_map {}", e)
                            }
                        }

                        print(&format!(
                            "Grabbed {} in {:?} ms",
                            p.path,
                            window().performance().unwrap().now() - start
                        ));

                        // traverses through object and download dependencies
                        match node_data.children.get("back") {
                            None => {}
                            Some(img_back) => {
                                let layers_back = img_back
                                    .children
                                    .keys()
                                    .filter(|x| x.parse::<u16>().is_ok())
                                    .collect::<Vec<&String>>();

                                for layer in layers_back {
                                    let tmp_node = img_back.children.get(layer).unwrap();

                                    match tmp_node.children.get("bS") {
                                        None => {}
                                        Some(b_s) => match &b_s.data {
                                            NodeDataPopulated::String(s) => {
                                                if s.len() > 0 {
                                                    imgs_to_grab
                                                        .insert(format!("Map.nx/Back/{}.img", *s));
                                                }
                                            }
                                            _ => {
                                                panic!("Unexpected other types!!!")
                                            }
                                        },
                                    }
                                }

                                let layers = node_data
                                    .children
                                    .keys()
                                    .filter(|x| x.parse::<u16>().is_ok())
                                    .collect::<Vec<&String>>();

                                for layer in layers {
                                    match node_data.children[layer].children.get("obj") {
                                        None => {}
                                        Some(node) => {
                                            let tmp_keys = node
                                                .children
                                                .keys()
                                                .filter(|x| x.parse::<u16>().is_ok())
                                                .collect::<Vec<&String>>();

                                            for key in tmp_keys {
                                                let tmp_node = node.children.get(key).unwrap();

                                                imgs_to_grab.insert(format!(
                                                    "Map.nx/Obj/{}.img",
                                                    match &tmp_node.children.get("oS").unwrap().data
                                                    {
                                                        NodeDataPopulated::String(s) => {
                                                            (*s).clone()
                                                        }
                                                        _ => {
                                                            panic!("Unexpected other types!!!")
                                                        }
                                                    }
                                                ));
                                            }
                                        }
                                    }

                                    // get tile img
                                    if let Some(info) =
                                        node_data.children[layer].children.get("info")
                                    {
                                        if let Some(node) = info.children.get("tS") {
                                            match &node.data {
                                                NodeDataPopulated::String(s) => {
                                                    imgs_to_grab.insert(format!(
                                                        "Map.nx/Tile/{}.img",
                                                        (*s).clone()
                                                    ));
                                                }
                                                _ => {
                                                    panic!("Unexpected data type in info attribute")
                                                }
                                            }
                                        }
                                    }

                                    match node_data.children["info"].children.get("bgm") {
                                        None => {}
                                        Some(info) => match &info.data {
                                            NodeDataPopulated::String(s) => {
                                                let parts = s.split("/").collect::<Vec<&str>>();
                                                imgs_to_grab.insert(format!(
                                                    "Sound.nx/{}.img/{}",
                                                    parts[0], parts[1]
                                                ));
                                            }
                                            _ => {
                                                panic!("Missing data")
                                            }
                                        },
                                    }
                                }
                                print(&format!("{:?}", imgs_to_grab));
                            }
                        }
                    }

                    WSResponse::Error(err) => {
                        return Err(format!("{}", err));
                    }
                    WSResponse::Message(msg) => {
                        print(&format!("Got message {}", msg));
                    }
                }
            }
            Err(e) => {
                let error_string = format!("Error getting lock {}", e);
                print(&error_string);
                return Err(error_string);
            }
        }
    }
    let mut tmp_node_editor = tmp_h_m_clone.lock().unwrap();
    *tmp_node_editor = WSResponse::Empty;

    print(" ");
    Ok(imgs_to_grab.into_iter().collect::<Vec<String>>().clone())
}
