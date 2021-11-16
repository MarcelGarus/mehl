pub trait DestructureTuple<T> {
    fn tuple2(self) -> Option<(T, T)>;
}
impl<T> DestructureTuple<T> for Vec<T> {
    fn tuple2(self) -> Option<(T, T)> {
        if self.len() != 2 {
            None
        } else {
            let mut iter = self.into_iter();
            let first = iter.next().unwrap();
            let second = iter.next().unwrap();
            assert!(matches!(iter.next(), None));
            Some((first, second))
        }
    }
}

pub trait RemoveLast<T> {
    fn remove_last(&mut self) -> T;
}
impl<T> RemoveLast<T> for Vec<T> {
    fn remove_last(&mut self) -> T {
        self.remove(self.len() - 1)
    }
}
