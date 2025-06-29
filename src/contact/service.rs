use crate::{integration::cache, user};

use super::{
    Id, Repository, Status, StatusTransition,
    model::{Contact, ContactDto, NewContact},
};

pub trait ContactService {
    fn find(&self, auth_id: &user::Id, recipient: &user::Id) -> super::Result<Option<ContactDto>>;

    fn find_by_id(&self, auth_id: &user::Id, id: &Id) -> super::Result<ContactDto>;

    fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>>;

    fn find_by_user_id_and_status(
        &self,
        sub: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>>;

    fn add(&self, me: &user::Id, you: &user::Id) -> super::Result<Status>;

    fn transition_status(
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
    _redis: cache::Redis,
}

impl ContactServiceImpl {
    pub fn new(repo: Repository, redis: cache::Redis) -> Self {
        Self {
            repo,
            _redis: redis,
        }
    }
}

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

    fn find_by_user_id(&self, user_id: &user::Id) -> super::Result<Vec<ContactDto>> {
        // TODO: cache contacts
        // let contacts = self
        //     .redis
        //     .smembers::<HashSet<Sub>>(cache::Key::Contacts(sub.to_owned()))
        //     .await;

        // match contacts {
        //     Some(c) => Ok(c),
        //     None => self.cache_contacts(sub).await,
        // }

        let contacts = self.repo.find_by_user_id(user_id)?;
        let dtos = contacts
            .iter()
            .map(|c| map_to_dto(user_id, c))
            .collect::<Vec<_>>();

        Ok(dtos)
    }

    fn find_by_user_id_and_status(
        &self,
        user_id: &user::Id,
        s: &Status,
    ) -> super::Result<Vec<ContactDto>> {
        // TODO: cache contacts
        let contacts = self.repo.find_by_user_id_and_status(user_id, s)?;

        let dtos = contacts
            .iter()
            .map(|c| map_to_dto(user_id, c))
            .collect::<Vec<_>>();

        Ok(dtos)
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

    fn transition_status(
        &self,
        auth_id: &user::Id,
        id: &Id,
        st: StatusTransition<'_>,
    ) -> super::Result<Status> {
        let contact = self.repo.find_by_id(id)?;
        match contact {
            Some(c) => {
                let mut dto = map_to_dto(auth_id, &c);
                let s = dto.transition(st)?;
                self.repo.update_status(c.id(), &s)?;
                Ok(s)
            }
            None => return Err(super::Error::NotFound(id.clone())),
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
