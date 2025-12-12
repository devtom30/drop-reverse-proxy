use drop_reverse_proxy::IpRepo;

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
