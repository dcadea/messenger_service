#[cfg(test)]
mod tests {
    use redis::Commands;
    use messenger_api::cache::client::init_redis;

    #[tokio::test]
    async fn test_set() {
        let mut con = init_redis().await;
        let _: () = con.set("my_key", 42).unwrap();

        let result: i32 = con.get("my_key").unwrap();
        assert_eq!(result, 42);

        let _: () = con.del("my_key").unwrap();
    }

    #[tokio::test]
    async fn remove_all_keys() {
        let mut con = init_redis().await;

        let keys: Vec<String> = con.keys("*").unwrap();

        keys.iter().for_each(|key: &String| {
            println!("Deleting key: {}", key);
            let _: () = con.del(key).unwrap();
        });
    }
}