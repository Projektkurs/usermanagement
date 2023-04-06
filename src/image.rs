use crate::debug_println;
use crate::user;
use crate::MainDatabase;
use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::Route;
use rocket_db_pools::Connection;
use rocket_multipart_form_data::{
    mime, MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
#[derive(Debug, Serialize, Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[post("/upload", data = "<data>")]
async fn upload(
    content_type: &ContentType,
    data: rocket::Data<'_>,
    db: Connection<MainDatabase>,
) -> Option<()> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("image")
            .size_limit(1024 * 200 * 1024)
            .content_type_by_string(Some(mime::IMAGE_STAR))
            .unwrap(),
        MultipartFormDataField::text("username"),
        MultipartFormDataField::text("password"),
    ]);

    let mut multipart_form_data = MultipartFormData::parse(content_type, data, options)
        .await
        .unwrap();

    let photo = multipart_form_data.files.get("image")?;
    let username = multipart_form_data.texts.remove("username")?.remove(0);
    let password = multipart_form_data.texts.remove("password")?.remove(0);
    debug_println!("login");
    let _logged_in_user = user::User::login(&username.text, password.text, &*db)
        .await
        .ok()?;

    let file_field = &photo[0];

    let _content_type = &file_field.content_type;
    let _file_name = file_field.file_name.clone()?;
    let _path = &file_field.path;
    {
        debug_println!("{}", _file_name);
        debug_println!("path:{}", _path.display());
        let mut src_file = File::open(_path).ok()?;
        debug_println!("opened src_file");
        let mut dst_file = File::create(format!("./images/{}", _file_name)).ok()?;
        debug_println!("opened dst_file");
        std::io::copy(&mut src_file, &mut dst_file).ok()?;
    }
    debug_println!("create preview");
    let img = image::open(format!("./images/{}", _file_name)).ok()?;
    //let img = image::open(_path).ok()?;
    debug_println!("opened preview");
    let preview_img = img.thumbnail(300, 300);
    let preview_path = format!("./previews/{}", _file_name);
    let mut preview_file = File::create(&preview_path).ok()?;
    debug_println!("createt preview file");
    preview_img
        .write_to(&mut preview_file, image::ImageOutputFormat::Jpeg(40))
        .ok()?;
    Some(())
}

#[post("/list", data = "<form>")]
async fn list(form: Form<user::UserData<'_>>, db: Connection<MainDatabase>) -> Option<String> {
    let _logged_in_user = user::User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    let dir = std::fs::read_dir(Path::new("./images")).ok()?;
    let mut names: Vec<String> = Vec::new();
    for file in dir.into_iter() {
        names.push(file.ok()?.file_name().into_string().ok()?);
    } //TODO: change it do be a string to begin with
    let mut ret = String::new();
    for file_name in names {
        ret.push_str(&file_name);
        ret.push_str("\n");
    }
    ret.pop();
    Some(ret)
}
async fn get_with_directory(
    directory: &Path,
    image: &str,
    form: Form<user::UserData<'_>>,
    db: Connection<MainDatabase>,
) -> Option<NamedFile> {
    let _logged_in_user = user::User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    let path_name = directory.join(Path::new(image));
    let path = Path::new(&path_name);
    if path.is_dir() {
        return None;
    }

    NamedFile::open(path).await.ok()
}

#[post("/get/<image>", data = "<form>")]
async fn get(
    image: &str,
    form: Form<user::UserData<'_>>,
    db: Connection<MainDatabase>,
) -> Option<NamedFile> {
    get_with_directory(Path::new("./images/"), image, form, db).await
}
#[post("/preview/<image>", data = "<form>")]
async fn preview(
    image: &str,
    form: Form<user::UserData<'_>>,
    db: Connection<MainDatabase>,
) -> Option<NamedFile> {
    get_with_directory(Path::new("./previews/"), image, form, db).await
}
pub fn routes() -> Vec<Route> {
    routes![upload, list, get, preview]
}
