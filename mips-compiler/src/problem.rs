
trait Zesty
{
    fn woop(&self);
}


enum A
{
    Happy(Box<dyn Zesty>),
    Nihilstic,
    Sad(String)
}

impl <T:Zesty+'static> From<Result<T, String>> for A
{
    fn from(src: Result<T, String>) -> Self {
        match src {
            Ok(x) =>         A::Happy(Box::new(x)),
            Err(_) => A::Sad("sad".to_string()),
        }

    }
}