use shared::config::{ActorConfiguration, ActorType};

pub fn create_config(hostname: &str, verify_ssl: bool) -> ActorConfiguration {
    ActorConfiguration {
        broker_url: format!("https://{hostname}/uds/rest/"),
        verify_ssl,
        actor_type: Some(ActorType::Managed),
        master_token: None,
        own_token: None,
        restrict_net: None,
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: 0,
        config: None,
        data: None,
    }
}