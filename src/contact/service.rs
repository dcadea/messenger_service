use crate::{integration::cache, user};

use super::{
    Id, Repository, Status, StatusTransition,
    model::{Contact, ContactDto, Contacts, NewContact},
};

#[async_trait::async_trait]
pub trait ContactService {
    fn find(&self, auth_id: &user::Id, recipient: &user::Id) -> super::Result<Option<ContactDto>>;

    fn find_by_id(&self, auth_id: &user::Id, id: &Id) -> super::Result<ContactDto>;

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>>;

    async fn find_by_user_id_and_status(
        &self,
        sub: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>>;

    fn add(&self, me: &user::Id, you: &user::Id) -> super::Result<Status>;

    async fn transition_status(
        &self,
        auth_id: &user::Id,
        id: &Id,
        t: StatusTransition<'_>,
    ) -> super::Result<Status>;

    fn delete(&self, auth_id: &user::Id, contact: &user::Id) -> super::Result<()>;
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
    fn find(&self, auth_id: &user::Id, recipient: &user::Id) -> super::Result<Option<ContactDto>> {
        if auth_id.eq(recipient) {
            return Err(super::Error::SameUsers(auth_id.clone()));
        }

        // TODO: cache
        self.repo
            .find(auth_id, recipient)
            .map(|c| c.map(|c| map_to_dto(auth_id, &c)))
    }

    fn find_by_id(&self, auth_id: &user::Id, id: &Id) -> super::Result<ContactDto> {
        // TODO: cache
        let c = self.repo.find_by_id(id)?;

        c.map(|c| map_to_dto(auth_id, &c))
            .ok_or(super::Error::NotFound(id.clone()))
    }

    async fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>> {
        let contacts = self
            .redis
            .json_get::<Contacts>(cache::Key::Contacts(user_id), None)
            .await;

        match contacts {
            Some(c) => Ok(c.get().clone()),
            None => self.cache_contacts(user_id).await,
        }
    }

    async fn find_by_user_id_and_status(
        &self,
        user_id: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>> {
        let path = format!(r#"$.[?(@.status.indicator == "{}")]"#, s.as_str());
        let contacts = self
            .redis
            .json_get::<Contacts>(cache::Key::Contacts(user_id), Some(&path))
            .await;

        if let Some(c) = contacts {
            Ok(c.get().clone())
        } else {
            let c = self
                .cache_contacts(user_id)
                .await?
                .into_iter()
                .filter(|c| c.status().eq(s))
                .collect::<Vec<_>>();
            Ok(c)
        }
    }

    fn add(&self, me: &user::Id, you: &user::Id) -> super::Result<Status> {
        if me.eq(you) {
            return Err(super::Error::SameUsers(me.clone()));
        }

        let exists = self.repo.exists(me, you)?;
        if exists {
            return Err(super::Error::AlreadyExists);
        }

        self.repo.add(&NewContact::new(me, you))?;
        // tokio::try_join!(
        // TODO
        // self.cache_contacts(&c.sub1),
        // self.cache_contacts(&c.sub2)
        // )?;

        Ok(Status::Pending {
            initiator: me.clone(),
        })
    }

    async fn transition_status(
        &self,
        auth_id: &user::Id,
        id: &Id,
        st: StatusTransition<'_>,
    ) -> super::Result<Status> {
        let contact = self.repo.find_by_id(id)?;
        match contact {
            Some(c) => {
                let dto = map_to_dto(auth_id, &c);
                let s = dto.transition(st)?;
                self.repo.update_status(c.id(), &s)?;
                self.invalidate(&c).await;
                Ok(s)
            }
            None => Err(super::Error::NotFound(id.clone())),
        }
    }

    fn delete(&self, auth_id: &user::Id, contact: &user::Id) -> super::Result<()> {
        assert_ne!(auth_id, contact);

        self.repo.delete(auth_id, contact)?;

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
    async fn cache_contacts(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>> {
        let contacts = self.repo.find_by_user_id(user_id)?;

        if contacts.is_empty() {
            return Ok(Vec::with_capacity(0));
        }

        let contacts = contacts
            .iter()
            .map(|c| map_to_dto(user_id, c))
            .collect::<Vec<_>>();

        let _: () = self
            .redis
            .json_set_ex(cache::Key::Contacts(user_id), &contacts)
            .await;

        Ok(contacts)
    }

    async fn invalidate(&self, c: &Contact) {
        tokio::join!(
            self.redis.json_del(cache::Key::Contacts(c.user_id_1())),
            self.redis.json_del(cache::Key::Contacts(c.user_id_2()))
        );
    }
}

fn map_to_dto(auth_id: &user::Id, c: &Contact) -> ContactDto {
    let (sender, recipient) = if auth_id.eq(c.user_id_1()) {
        (c.user_id_1(), c.user_id_2())
    } else {
        (c.user_id_2(), c.user_id_1())
    };

    ContactDto::new(
        c.id().clone(),
        sender.clone(),
        recipient.clone(),
        Status::from(c),
    )
}
