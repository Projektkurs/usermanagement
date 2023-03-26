use crate::{
    database::{Connection, DatabaseUtils, MainDatabase},
    room::Room,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use bson::oid::ObjectId;
use mongodb::bson::doc;
use rocket::form::Form;
use rocket::Route;
use serde::*;
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id")]
    id: bson::oid::ObjectId,
    // todo: username as id
    //#[serde(rename = "_id")]
    username: String,
    password_hash: String,
    is_active: bool,
    // Permissions
    is_admin: bool,
    can_creat_users: bool,
    editable_rooms: Vec<bson::oid::ObjectId>, // saves name of the room
    // Personal information
    firstname: String,
    surname: String,
    email: Option<String>,
    phone_number: Option<String>, // String for setting prefix in brackets or +49
}

impl<'a> User {
    // why does this function take so long to excecute? is it the SaltString?
    pub fn new(firstname: String, surname: String, password: String) -> Result<Self, &'a str> {
        if password == " " {
            return Err("Password cannot be empty");
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let user = Ok(User {
            id: bson::oid::ObjectId::new(),
            username: format!("{}.{}", surname, firstname),
            password_hash,
            is_active: true,
            is_admin: false,
            can_creat_users: false,
            editable_rooms: Vec::new(),
            firstname,
            surname,
            email: None,
            phone_number: None,
        });
        user.validate_first_name().validate_surname()
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    async fn insert(&self, db: &mongodb::Client) -> Option<()> {
        if let Some(_) = User::getfromdb(&self.username, &db).await {
            return None;
        }
        //todo
        if let Ok(_user) = db.user_collection().insert_one(self, None).await {
            Some(())
        } else {
            None
        }
    }
    async fn update(&self, db: &mongodb::Client) -> Option<()> {
        if db
            .user_collection()
            .replace_one(doc! {"_id":self.id}, self, None)
            .await
            .is_ok()
        {
            Some(())
        } else {
            None
        }
    }
    async fn getfromdb(username: &str, db: &mongodb::Client) -> Option<Self> {
        db.user_collection()
            .find_one(doc! {"username": username}, None)
            .await
            .unwrap()
    }
    // TODO: remove timing attack as the time if the user is right is significantly longer
    pub async fn login(
        username: &str,
        password: String,
        db: &mongodb::Client,
    ) -> Result<Self, &'a str> {
        let user = db
            .user_collection()
            .find_one(doc! {"username": username}, None)
            .await
            .unwrap();
        if let Some(user) = user {
            let passwordhash = PasswordHash::new(&user.password_hash).unwrap();

            if let Ok(_) = Argon2::default().verify_password(password.as_bytes(), &passwordhash) {
                return Ok(user);
            } else {
                return Err("password wrong");
            }
        } else {
            return Err("8ser not known");
        }
    }
    pub fn can_edit_room(&self, room_id: &ObjectId) -> bool {
        if self.is_admin {
            return true;
        }
        self.editable_rooms.contains(&room_id)
    }
    pub fn can_create_rooms(&self) -> bool {
        self.is_admin
    }
}

trait UserValidate<'a>
where
    Self: Sized,
{
    fn validate_first_name(self) -> Self;
    fn validate_surname(self) -> Self;
}
impl<'a> UserValidate<'a> for Result<User, &'a str> {
    // TODO look for a way to do normal error propagation while holding onto the ownership
    fn validate_first_name(self) -> Self {
        if self.is_err() {
            return self;
        }
        if self.as_ref().unwrap().firstname == "" {
            return Err("the first name cannot be empty");
        }
        self
    }
    fn validate_surname(self) -> Self {
        if self.is_err() {
            return self;
        }
        if self.as_ref().unwrap().surname == "" {
            return Err("the surname cannot be empty");
        }
        self
    }
}

#[derive(Debug, FromForm)]
pub struct UserData<'r> {
    pub username: &'r str,
    pub password: &'r str,
}
#[derive(Debug, FromForm)]
struct CreateUserForm<'r> {
    userdata: UserData<'r>,
    firstname: &'r str,
    surname: &'r str,
    password: &'r str,
}
// todo: get access rights
// add_room
// todo: why does this function take so long to excecute?
#[post("/create", data = "<form>")]
async fn create(form: Form<CreateUserForm<'_>>, db: Connection<MainDatabase>) -> String {
    println!("{:?}", form);
    let logged_in_user = User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await;
    if logged_in_user.is_ok() {
        let logged_in_user = logged_in_user.unwrap();
        if !(logged_in_user.is_admin) && !(logged_in_user.can_creat_users) {
            return String::from("you need to be admin or be able to create users");
        }
        let new_user = User::new(
            String::from(form.firstname),
            String::from(form.surname),
            String::from(form.password),
        );
        if let Ok(new_user) = new_user {
            new_user.insert(&*db).await;
            return rocket::serde::json::to_string(&new_user).unwrap();
        } else {
            return String::from(new_user.err().unwrap());
        }
    } else {
        return String::from(logged_in_user.err().unwrap());
    }
}
#[derive(Debug, FromForm)]
struct DeleteUserForm<'r> {
    userdata: UserData<'r>,
    name: &'r str,
}
#[post("/delete", data = "<form>")]
async fn delete(form: Form<DeleteUserForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    let user = User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    if !user.is_admin {
        return None;
    }
    db.user_collection()
        .delete_one(doc! {"_id": form.name}, None)
        .await
        .ok()?;
    Some(())
}

#[post("/isvalid", data = "<form>")]
async fn isvalid(form: Form<UserData<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    let _user = User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    Some(())
}
#[post("/get", data = "<form>")]
async fn get(form: Form<UserData<'_>>, db: Connection<MainDatabase>) -> Option<String> {
    rocket::serde::json::to_string(
        &User::login(form.username, String::from(form.password), &*db)
            .await
            .ok()?,
    )
    .ok()
}
#[derive(Debug, FromForm)]
struct ChangeUserForm<'r> {
    userdata: UserData<'r>,
    email: Option<String>,
    firstname: Option<String>,
    surname: Option<String>,
    phone_number: Option<String>,
}
#[post("/change", data = "<form>")]
async fn change(form: Form<ChangeUserForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    let mut user = User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    if let Some(firstname) = form.firstname.clone() {
        user.firstname = firstname;
    }
    if let Some(surname) = form.surname.clone() {
        user.surname = surname;
    }
    user.email = form.email.clone();
    user.phone_number = form.phone_number.clone();
    user.update(&db).await?;
    Some(())
}
#[post("/get_rooms", data = "<form>")]
async fn get_rooms(form: Form<UserData<'_>>, db: Connection<MainDatabase>) -> Option<String> {
    //rocket::serde::json::to_string(
    let user = User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    let mut ret = String::new();
    if user.is_admin {
        let mut rooms = Room::get_all_from_db(&db).await?;
        while rooms.advance().await.ok()? {
            let room = rooms.deserialize_current().ok()?;
            ret.push_str(room.get_name());
            ret.push_str("\n");
        }
    } else {
        for room_id in user.editable_rooms {
            let room = Room::getfromdb_id(&room_id, &db).await?;
            ret.push_str(room.get_name());
            ret.push_str("\n");
        }
    }
    Some(ret)
}

#[derive(Debug, FromForm)]
struct UpdatePasswordForm<'a> {
    userdata: UserData<'a>,
    new_password: String,
}
#[post("/update_password", data = "<form>")]
async fn update_password(
    form: Form<UpdatePasswordForm<'_>>,
    db: Connection<MainDatabase>,
) -> Option<()> {
    if form.new_password == "" {
        return None;
    }

    let mut user = User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(form.new_password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    user.password_hash = password_hash;
    user.update(&*db).await?;
    Some(())
}

pub fn routes() -> Vec<Route> {
    routes![
        create,
        delete,
        get,
        get_rooms,
        isvalid,
        update_password,
        change
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::Client;
    use tokio::time;
    /// create some demo users on the life database.
    #[tokio::test]
    async fn test_usercreation() {
        let mut response_times = Vec::new();

        let client = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .unwrap();
        let users = &mut Vec::new();
        let start_time = time::Instant::now();

        users.push(
            User::new(
                String::from("Max"),
                String::from("Mustermann"),
                String::from("1234"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Anna"),
                String::from("Müller"),
                String::from("5678"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Hans"),
                String::from("Schmidt"),
                String::from("9012"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Maria"),
                String::from("Gonzalez"),
                String::from("3456"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Thomas"),
                String::from("Lee"),
                String::from("7890"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Sarah"),
                String::from("Kim"),
                String::from("2345"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("David"),
                String::from("Garcia"),
                String::from("6789"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Julia"),
                String::from("Chen"),
                String::from("0123"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Peter"),
                String::from("Jürgensen"),
                String::from("4567"),
            )
            .unwrap(),
        );
        users.push(
            User::new(
                String::from("Lisa"),
                String::from("Wong"),
                String::from("8901"),
            )
            .unwrap(),
        );

        let end_time = time::Instant::now();
        let response_time = end_time - start_time;
        response_times.push(response_time);
        let start_time = time::Instant::now();

        let insertfuture: &mut Vec<
            std::pin::Pin<Box<dyn std::future::Future<Output = Option<()>>>>,
        > = &mut Vec::new();
        for user in users {
            insertfuture.push(Box::pin(user.insert(&client)));
        }
        let end_time = time::Instant::now();
        let response_time = end_time - start_time;
        response_times.push(response_time);
        for future in insertfuture {
            let start_time = time::Instant::now();

            future.await;
            let end_time = time::Instant::now();
            let response_time = end_time - start_time;
            response_times.push(response_time);
        }

        println!("Average response time: {:?}", response_times);
    }
}
