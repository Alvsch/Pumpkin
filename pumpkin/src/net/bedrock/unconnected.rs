use std::net::SocketAddr;

use pumpkin_config::BASIC_CONFIG;
use pumpkin_protocol::bedrock::{
    client::raknet::unconnected_pong::{CUnconnectedPong, ServerInfo},
    server::raknet::unconnected_ping::SUnconnectedPing,
};
use tokio::net::UdpSocket;

use crate::{
    net::bedrock::BedrockClient,
    server::{CURRENT_BEDROCK_MC_VERSION, Server},
};

impl BedrockClient {
    pub async fn handle_unconnected_ping(
        server: &Server,
        packet: SUnconnectedPing,
        addr: SocketAddr,
        socket: &UdpSocket,
    ) {
        // TODO
        let player_count = server
            .get_status()
            .lock()
            .await
            .status_response
            .players
            .as_ref()
            .unwrap()
            .online as _;

        let motd_string = ServerInfo {
            edition: "MCPE",
            // TODO The default motd is to long to be displayed completely
            motd_line_1: "Pumpkin Server",
            protocol_version: 819,
            version_name: CURRENT_BEDROCK_MC_VERSION,
            player_count,
            // A large number looks wreird on the client worlds window
            max_player_count: BASIC_CONFIG.max_players,
            server_unique_id: server.server_guid,
            motd_line_2: &BASIC_CONFIG.default_level_name,
            game_mode: server.defaultgamemode.lock().await.gamemode.to_str(),
            game_mode_numeric: 1,
            port_ipv4: 19132,
            port_ipv6: 19133,
        };
        Self::send_offline_packet(
            &CUnconnectedPong::new(
                packet.time,
                server.server_guid,
                packet.magic,
                format!("{motd_string}"),
            ),
            addr,
            socket,
        )
        .await;
    }
}
