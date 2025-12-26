use std::path::PathBuf;
use figment::{Figment, providers::{Format, Toml}};
use drop_reverse_proxy::{create_conf_from_toml_file, Conf, IpRepo};

#[test]
fn ip_repo_save_or_update_when_not_exists() {
    let ip_repo = drop_reverse_proxy::InMemoryIpRepo::default();
    ip_repo.save_or_update(&std::net::IpAddr::from([127,0,0,1]), 0);
    assert_eq!(0, *ip_repo.get(&std::net::IpAddr::from([127,0,0,1])).unwrap().nb_bad_attempts());
}

#[test]
fn ip_repo_save_or_update_when_exists() {
    let ip_repo = drop_reverse_proxy::InMemoryIpRepo::default();
    let ip = std::net::IpAddr::from([127,0,0,1]);
    ip_repo.save_or_update(&ip, 0);
    assert_eq!(0, *ip_repo.get(&std::net::IpAddr::from([127,0,0,1])).unwrap().nb_bad_attempts());
}

#[test]
fn ip_repo_save_or_update_when_exists_and_nb_bad_attempts_is_more_than_zero() {
    let ip_repo = drop_reverse_proxy::InMemoryIpRepo::default();
    let ip = std::net::IpAddr::from([127,0,0,1]);
    ip_repo.save_or_update(&ip, 1);
    assert_eq!(1, *ip_repo.get(&std::net::IpAddr::from([127,0,0,1])).unwrap().nb_bad_attempts());
}

#[test]
fn conf_is_ok() {
    let config_result = create_conf_from_toml_file("tests/resources/conf/ok/app.toml");

    assert!(config_result.is_ok());
    let config = config_result.unwrap();
    assert!(! config.tags().is_empty());
    assert_eq!(["jdznjevb", "xurnxenyoawltkky", "tag3"].to_vec(), *config.tags());
    assert_eq!("http://localhost:8084", config.redirect_uri());
    assert_eq!("127.0.0.1:8000", config.bind_addr());
    assert_eq!(10, config.max_attempts());
}
