pub trait SessionChangeNotification{
    type AccountMap;
    fn on_new_session(new: Self::AccountMap);
}