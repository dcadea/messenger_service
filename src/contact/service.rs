use std::collections::HashSet;

use crate::{integration::cache, user};

use super::{Repository, model::Contact};

#[async_trait::async_trait]
pub trait ContactService {
    async fn find_contact_subs(&self, sub: &user::Sub) -> super::Result<HashSet<user::Sub>>;

    async fn add(&self, c: &Contact) -> super::Result<()>;

    async fn delete(&self, auth_sub: &user::Sub, contact: &user::Sub) -> super::Result<()>;
}

#[derive(Clone)]
pub struct ContactServiceImpl {
    repo: Repository,
    redis: cache::Redis,
}

impl ContactServiceImpl {
    pub fn new(repo: Repository, redis: cache::Redis) -> Self {
        Self { repo, redis }
    }
}

#[async_trait::async_trait]
impl ContactService for ContactServiceImpl {
    async fn find_contact_subs(&self, sub: &user::Sub) -> super::Result<HashSet<user::Sub>> {
        let contacts = self
            .redis
            .smembers::<HashSet<user::Sub>>(cache::Key::Contacts(sub.to_owned()))
            .await;

        match contacts {
            Some(c) => Ok(c),
            None => self.cache_contacts(sub).await,
        }
    }

    async fn add(&self, c: &Contact) -> super::Result<()> {
        if c.sub1.eq(&c.sub2) {
            return Err(super::Error::SelfReference);
        }

        let exists = self.repo.exists(&c.sub1, &c.sub2).await?;
        if exists {
            return Err(super::Error::AlreadyExists(c.sub1.clone(), c.sub2.clone()));
        }

        tokio::try_join!(
            self.repo.add(c),
            self.cache_contacts(&c.sub1),
            self.cache_contacts(&c.sub2)
        )?;

        Ok(())
    }

    async fn delete(&self, auth_sub: &user::Sub, contact: &user::Sub) -> super::Result<()> {
        assert_ne!(auth_sub, contact);

        self.repo.delete(auth_sub, contact).await?;

        tokio::join!(
            self.redis.srem(
                cache::Key::Contacts(auth_sub.to_owned()),
                contact.to_owned()
            ),
            self.redis.srem(
                cache::Key::Contacts(contact.to_owned()),
                auth_sub.to_owned()
            )
        );

        Ok(())
    }
}

impl ContactServiceImpl {
    async fn cache_contacts(&self, sub: &user::Sub) -> super::Result<HashSet<user::Sub>> {
        let contacts = self.repo.find_by_sub(sub).await?;

        if contacts.is_empty() {
            return Ok(HashSet::with_capacity(0));
        }

        let contacts = contacts
            .iter()
            .map(|c| c.get_recipient(sub).clone())
            .collect::<HashSet<_>>();

        let _: () = self
            .redis
            .sadd(cache::Key::Contacts(sub.clone()), &contacts)
            .await;

        Ok(contacts.iter().cloned().collect::<HashSet<_>>())
    }
}
