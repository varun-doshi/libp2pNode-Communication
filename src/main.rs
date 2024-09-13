use std::{error::Error,  str::FromStr, time::Duration};

use futures::StreamExt;
use libp2p::{floodsub::{Floodsub,FloodsubEvent, Topic}, identity, mdns::{self as Mdns, Event as MdnsEvent},noise::Config as NoiseConfig, swarm::{self, behaviour, NetworkBehaviour}, tcp::Config as TcpConfig, yamux::Config as YamuxConfig, Multiaddr, PeerId, Swarm, SwarmBuilder};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::{io, io::AsyncBufReadExt, select};



pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

#[derive(Serialize,Deserialize,Debug)]
pub struct MessageStore{
    pub mmessage:String,
    pub peer:PeerId
}

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub floodsub:Floodsub,
   pub mdns:Mdns::tokio::Behaviour,
}

impl MyBehaviour{
    pub fn new(config:Mdns::Config,peer_id:PeerId)->Self{
        let mut floodsub = Floodsub::new(peer_id);
        floodsub.subscribe(TOPIC.clone()); 
        Self{
            floodsub,
            mdns:Mdns::tokio::Behaviour::new(config, peer_id).expect("can make mdns")
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
    Input(String),
}

impl From<FloodsubEvent> for Event{
    fn from(value: FloodsubEvent) -> Self {
        Self::Floodsub(value)
    }
}
impl From<MdnsEvent> for Event {
    fn from(event: MdnsEvent) -> Self {
        Self::Mdns(event)
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            TcpConfig::default(), 
            NoiseConfig::new, 
            YamuxConfig::default 
        )?
        .with_behaviour(|key| {

            let local_peer_id = PeerId::from(key.clone().public());
            
            let mut  behaviour = MyBehaviour::new(Mdns::Config::default(), local_peer_id);
           let subscribed_topic= behaviour.floodsub.subscribe(TOPIC.clone());
            println!("LocalPeerID: {local_peer_id} and CHAT:{subscribed_topic}");
            behaviour
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(30)))
        .build();

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        if let Some(addr) = std::env::args().nth(1) {
            let remote: Multiaddr = addr.parse()?;
            swarm.dial(remote)?;
            println!("Dialed {addr}")
        }

        loop{
            select! {
                Ok(Some(line)) = stdin.next_line() => {
                    if let Err(e) = handle_input_event(line,&mut swarm).await{
                        println!("Publish error: {e:?}");
                    }
                }

                event = swarm.select_next_some() => match event {

                    swarm::SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(Mdns::Event::Discovered(peers)))=>{
                        for (peer_id, _) in peers {
                            println!("Discovered peer: {:?}", peer_id);
                            swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                        }
                    },
                    // swarm::SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(Mdns::Event::Expired(peers)))=>{
                    //     for (peer_id, _) in peers {
                    //         println!("Discovered peer: {:?}", peer_id);
                    //         swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                    //     }
                    // },
                    swarm::SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(FloodsubEvent::Message(message))) => {
                        let msg_content = String::from_utf8_lossy(&message.data);
                        println!("Received message from {:?}, Message: {}", message.source, msg_content);
                    },
                    swarm::SwarmEvent::NewListenAddr { listener_id, address }=>{
                        println!("New node in network:{:?}",address);
                    },
                    swarm::SwarmEvent::ListenerClosed { listener_id, addresses, reason }=>{
                        println!("Node closed:{:?}",addresses[0]);
                        // swarm.behaviour_mut().floodsub.remove_node_from_partial_view(addresses[0]);
                    },
                    // swarm::SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(FloodsubEvent::))
                    _=>{}
            }
        }
    }

  
}


async fn handle_input_event(line: String, swarm: &mut Swarm<MyBehaviour>)->Result<(), Box<dyn std::error::Error>> {
    match line.as_str() {
        "ls p" => show_peers(swarm).await,
        cmd if cmd.starts_with("ls c") => check_has_peer(cmd, swarm).await,
 
        _ => {
            println!("Sending floodsub message");
            swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), line.as_bytes());
            println!("{:?}",swarm.connected_peers().collect::<Vec<_>>());
            
        }
    }
    Ok(())
}

pub async fn check_has_peer(line:&str,swarm:&mut Swarm<MyBehaviour>){
    let rest = line.strip_prefix("ls c ").expect("End of line");
    let peer_exists=swarm.behaviour().mdns.has_node(&PeerId::from_str(rest).expect("Invalid peerId"));
    match peer_exists{
        true=>println!("Peer exists"),
        false=>println!("Peer does not exist"),
    }
    
}
pub async fn show_peers(swarm:&mut Swarm<MyBehaviour>){
    let peers=swarm.behaviour().mdns.discovered_nodes().collect::<Vec<_>>();
    println!("Connected peers: {:?}",peers);
}