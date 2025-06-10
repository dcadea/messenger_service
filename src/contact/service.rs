use async_trait::async_trait;

use crate::{integration::cache, user};

use super::{
    Id, Repository, Status, StatusTransition,
    model::{Contact, ContactDto},
};

#[async_trait]
pub trait ContactService {
    async fn find(
        &self,
        auth_id: &user::Id,
        recipient: &user::Id,
    ) -> super::Result<Option<ContactDto>>;

    async fn find_by_id(&self, auth_id: &user::Id, id: &Id) -> super::Result<ContactDto>;

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>>;

    async fn find_by_user_id_and_status(
        &self,
        sub: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>>;

    async fn add(&self, c: &Contact) -> super::Result<()>;

    async fn transition_status(&self, id: &Id, t: StatusTransition<'_>) -> super::Result<Status>;

    async fn delete(&self, auth_id: &user::Id, contact: &user::Id) -> super::Result<()>;
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

#[async_trait]
impl ContactService for ContactServiceImpl {
    async fn find(
        &self,
        auth_id: &user::Id,
        recipient: &user::Id,
    ) -> super::Result<Option<ContactDto>> {
        if auth_id.eq(recipient) {
            return Err(super::Error::SameUsers(auth_id.clone()));
        }

        // TODO: cache
        self.repo
            .find(auth_id, recipient)
            .await
            .map(|c| c.map(|c| map_to_dto(auth_id, &c)))
    }

    async fn find_by_id(&self, auth_id: &user::Id, id: &Id) -> super::Result<ContactDto> {
        // TODO: cache
        let c = self.repo.find_by_id(id).await?;

        c.map(|c| map_to_dto(auth_id, &c))
            .ok_or(super::Error::NotFound(id.clone()))
    }

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>> {
        // TODO: cache contacts
        // let contacts = self
        //     .redis
        //     .smembers::<HashSet<Sub>>(cache::Key::Contacts(sub.to_owned()))
        //     .await;

        // match contacts {
        //     Some(c) => Ok(c),
        //     None => self.cache_contacts(sub).await,
        // }

        let contacts = self.repo.find_by_user_id(user_id).await?;
        let dtos = contacts
            .iter()
            .map(|c| map_to_dto(user_id, c))
            .collect::<Vec<_>>();

        Ok(dtos)
    }

    async fn find_by_user_id_and_status(
        &self,
        user_id: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>> {
        // TODO: cache contacts
        let contacts = self.repo.find_by_user_id_and_status(user_id, s).await?;

        let dtos = contacts
            .iter()
            .map(|c| map_to_dto(user_id, c))
            .collect::<Vec<_>>();

        Ok(dtos)
    }

    async fn add(&self, c: &Contact) -> super::Result<()> {
        if c.user_id1().eq(c.user_id2()) {
            return Err(super::Error::SameUsers(c.user_id1().clone()));
        }

        let exists = self.repo.exists(c.user_id1(), c.user_id2()).await?;
        if exists {
            return Err(super::Error::AlreadyExists(
                c.user_id1().clone(),
                c.user_id2().clone(),
            ));
        }

        tokio::try_join!(
            self.repo.add(c),
            // TODO
            // self.cache_contacts(&c.sub1),
            // self.cache_contacts(&c.sub2)
        )?;

        Ok(())
    }

    async fn transition_status(&self, id: &Id, st: StatusTransition<'_>) -> super::Result<Status> {
        let contact = self.repo.find_by_id(id).await?;
        match contact {
            Some(mut c) => {
                if !c.transition(st) {
                    return Err(super::Error::StatusTransitionFailed);
                }
                self.repo.update_status(&c).await?;
                Ok(c.status().clone())
            }
            None => return Err(super::Error::NotFound(id.clone())),
        }
    }

    async fn delete(&self, auth_id: &user::Id, contact: &user::Id) -> super::Result<()> {
        assert_ne!(auth_id, contact);

        self.repo.delete(auth_id, contact).await?;

        // FIXME:
        // tokio::join!(
        //     self.redis
        //         .srem(cache::Key::Contacts(auth_id), contact.to_owned()),
        //     self.redis
        //         .srem(cache::Key::Contacts(contact), auth_sub.to_owned())
        // );

        Ok(())
    }
}

impl ContactServiceImpl {
    // TODO: cache contacts
    // async fn cache_contacts(&self, sub: &Sub) -> super::Result<HashSet<Sub>> {
    //     let contacts = self.repo.find_by_sub(sub).await?;

    //     if contacts.is_empty() {
    //         return Ok(HashSet::with_capacity(0));
    //     }

    //     let contacts = contacts
    //         .iter()
    //         .map(|c| c.get_recipient(sub).clone())
    //         .collect::<HashSet<_>>();

    //     let _: () = self
    //         .redis
    //         .sadd(cache::Key::Contacts(sub.clone()), &contacts)
    //         .await;

    //     Ok(contacts.iter().cloned().collect::<HashSet<_>>())
    // }
}

fn map_to_dto(auth_id: &user::Id, c: &Contact) -> ContactDto {
    let recipient = if auth_id.eq(c.user_id1()) {
        c.user_id2()
    } else {
        c.user_id1()
    };

    ContactDto::new(c.id().clone(), recipient.clone(), c.status().clone())
}
