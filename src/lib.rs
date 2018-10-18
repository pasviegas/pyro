extern crate anymap;
extern crate itertools;
use anymap::{any::CloneAny, Map};
use itertools::{multizip, Zip};
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

pub struct World<S = SoaStorage> {
    storages: Vec<S>,
}

impl<S> World<S>
where
    S: Storage + RegisterComponent + Clone,
{
    pub fn new() -> Self {
        World {
            storages: Vec::new(),
        }
    }
    // [FIXME]: Why can't we use a impl trait here? An impl trait here results in lifetime issues.
    pub fn matcher<'s, Q>(&'s mut self) -> Box<Iterator<Item = <Q::Iter as Iterator>::Item> + 's>
    where
        Q: Query<'s>,
    {
        Box::new(
            self.storages
                .iter_mut()
                .filter_map(|storage| Q::query(storage))
                .flat_map(|iter| iter),
        )
    }
    pub fn add_entity<A: AppendComponents, I>(&mut self, i: I)
    where
        I: IntoIterator<Item = A>,
    {
        if let Some(storage) = self
            .storages
            .iter_mut()
            .find(|storage| A::is_match::<S>(storage))
        {
            A::append_components(i, storage);
        } else {
            let mut storage = A::ComponentList::build::<S>().access();
            A::append_components(i, &mut storage);
            self.storages.push(storage);
        }
    }
}
pub trait Component: 'static {}
impl<C: 'static> Component for C {}

pub type StorageId = u32;
pub type ComponentId = u32;

pub struct StorageBuilder<S: Storage> {
    current_id: ComponentId,
    storage_register: HashMap<ComponentId, S>,
}

impl<S> StorageBuilder<S>
where
    S: Storage,
{
    pub fn new() -> Self {
        StorageBuilder {
            current_id: 0,
            storage_register: HashMap::new(),
        }
    }

    pub fn add_storage(&mut self, storage: S) -> ComponentId {
        let id = self.current_id + 1;
        self.storage_register.insert(id, storage);
        self.current_id = id;
        id
    }

    // pub fn extent_from_storage<C: Component>(&mut self, id: ComponentId) -> S {
    //     let mut s = self
    //         .storage_register
    //         .get(&id)
    //         .expect("Id not found")
    //         .clone();
    //     s.register_component::<C>();
    //     s
    // }
}
pub trait Storage: Sized {
    fn empty() -> EmptyStorage<Self>;
    unsafe fn component<T: Component>(&self) -> Option<&[T]>;
    unsafe fn component_mut<T: Component>(&self) -> Option<&mut [T]>;
    fn append_components<I, A>(&mut self, components: I)
    where
        A: AppendComponents,
        I: IntoIterator<Item = A>;
    fn push_component<C: Component>(&mut self, component: C);
    fn contains<C: Component>(&self) -> bool;
    fn types(&self) -> &HashSet<TypeId>;
}

pub struct Exact<'s, Tuple>(pub PhantomData<&'s Tuple>);

impl<'s, A, B> Matcher for Exact<'s, (A, B)>
where
    A: Fetch<'s>,
    B: Fetch<'s>,
{
    fn is_match<S: Storage>(storage: &S) -> bool {
        let types = storage.types();
        types.len() == 2
            && types.contains(&TypeId::of::<A::Component>())
            && types.contains(&TypeId::of::<B::Component>())
    }
}
pub struct All<'s, Tuple>(pub PhantomData<&'s Tuple>);

pub trait ReadComponent {
    type Component: Component;
}
pub trait WriteComponent {
    type Component: Component;
}
pub struct Read<C>(PhantomData<C>);
impl<C: Component> ReadComponent for Read<C> {
    type Component = C;
}

impl<C> Read<C> {
    pub fn new() -> Self {
        Read(PhantomData)
    }
}
impl<C: Component> WriteComponent for Write<C> {
    type Component = C;
}

pub struct Write<C>(PhantomData<C>);
pub trait Slice {
    fn len(&self) -> usize;
}
impl<T> Slice for &[T] {
    fn len(&self) -> usize {
        self.len()
    }
}
impl<T> Slice for &mut [T] {
    fn len(&self) -> usize {
        self.len()
    }
}
pub trait Fetch<'s> {
    type Component: Component;
    type Iter: Iterator;
    unsafe fn fetch<S: Storage>(storage: &'s S) -> Option<Self::Iter>;
}

impl<'s, C: Component> Fetch<'s> for Read<C> {
    type Component = C;
    type Iter = std::slice::Iter<'s, C>;
    unsafe fn fetch<S: Storage>(storage: &'s S) -> Option<Self::Iter> {
        storage.component::<C>().map(|slice| slice.iter())
    }
}

impl<'s, C: Component> Fetch<'s> for Write<C> {
    type Component = C;
    type Iter = std::slice::IterMut<'s, C>;
    unsafe fn fetch<S: Storage>(storage: &'s S) -> Option<Self::Iter> {
        storage.component_mut::<C>().map(|slice| slice.iter_mut())
    }
}

pub trait Matcher {
    fn is_match<S: Storage>(storage: &S) -> bool;
}
pub trait Query<'s> {
    type Iter: Iterator + 's;
    fn query<S: Storage>(storage: &'s mut S) -> Option<Self::Iter>;
}

impl<'s, A, B> Matcher for All<'s, (A, B)>
where
    A: Fetch<'s>,
    B: Fetch<'s>,
{
    fn is_match<S: Storage>(storage: &S) -> bool {
        storage.contains::<A::Component>() && storage.contains::<B::Component>()
    }
}

impl<'s, A, B> Query<'s> for All<'s, (A, B)>
where
    A: Fetch<'s>,
    B: Fetch<'s>,
{
    type Iter = Zip<(A::Iter, B::Iter)>;
    fn query<S: Storage>(storage: &'s mut S) -> Option<Self::Iter> {
        unsafe {
            let i1 = A::fetch(storage)?;
            let i2 = B::fetch(storage)?;
            Some(multizip((i1, i2)))
        }
    }
}

pub struct EmptyStorage<S> {
    storage: S,
}

pub trait BuildStorage {
    fn build<S: Storage + Clone + RegisterComponent>() -> EmptyStorage<S>;
}

impl<A, B> BuildStorage for (A, B)
where
    A: Component,
    B: Component,
{
    fn build<S: Storage + Clone + RegisterComponent>() -> EmptyStorage<S> {
        S::empty()
            .register_component::<A>()
            .register_component::<B>()
    }
}

impl<S> EmptyStorage<S>
where
    S: Storage + Clone + RegisterComponent,
{
    pub fn register_component<C: Component>(&self) -> EmptyStorage<S> {
        let mut storage = self.storage.clone();
        storage.register_component::<C>();
        EmptyStorage { storage }
    }
    pub fn access(self) -> S {
        self.storage
    }
}

pub struct UnsafeStorage<T>(UnsafeCell<Vec<T>>);
impl<T> UnsafeStorage<T> {
    pub fn new() -> Self {
        UnsafeStorage(UnsafeCell::new(Vec::<T>::new()))
    }
    pub fn push(&self, t: T) {
        unsafe { (*self.0.get()).push(t) }
    }
    pub fn is_empty(&self) -> bool {
        unsafe { (*self.0.get()).is_empty() }
    }

    pub unsafe fn get_slice(&self) -> &[T] {
        (*self.0.get()).as_slice()
    }

    pub unsafe fn get_mut_slice(&self) -> &mut [T] {
        (*self.0.get()).as_mut_slice()
    }
}

impl<T> Clone for UnsafeStorage<T> {
    fn clone(&self) -> Self {
        assert!(self.is_empty());
        UnsafeStorage::new()
    }
}

pub trait ComponentList {
    const SIZE: usize;
    type Components;
}
impl<A, B> ComponentList for (A, B) {
    const SIZE: usize = 2;
    type Components = (A, B);
}
pub trait AppendComponents: Sized {
    type ComponentList: ComponentList + BuildStorage;
    fn is_match<S: Storage>(storage: &S) -> bool;
    fn append_components<I, S>(items: I, storage: &mut S)
    where
        S: Storage,
        I: IntoIterator<Item = Self>;
}

impl<A, B> AppendComponents for (A, B)
where
    A: Component,
    B: Component,
{
    type ComponentList = (A, B);
    fn is_match<S: Storage>(storage: &S) -> bool {
        let types = storage.types();
        types.len() == 2 && types.contains(&TypeId::of::<A>()) && types.contains(&TypeId::of::<B>())
    }

    fn append_components<I, S>(items: I, storage: &mut S)
    where
        S: Storage,
        I: IntoIterator<Item = Self>,
    {
        for (a, b) in items {
            storage.push_component(a);
            storage.push_component(b);
        }
    }
}

#[derive(Clone)]
pub struct SoaStorage {
    types: HashSet<TypeId>,
    anymap: Map<CloneAny>,
}

pub trait RegisterComponent {
    fn register_component<C: Component>(&mut self);
}

impl RegisterComponent for SoaStorage {
    fn register_component<C: Component>(&mut self) {
        self.types.insert(TypeId::of::<C>());
        self.anymap.insert(UnsafeStorage::<C>::new());
    }
}

impl Storage for SoaStorage {
    fn push_component<C: Component>(&mut self, component: C) {
        let storage = self
            .anymap
            .get_mut::<UnsafeStorage<C>>()
            .expect("Component not found");
        storage.push(component);
    }
    fn append_components<I, A>(&mut self, components: I)
    where
        A: AppendComponents,
        I: IntoIterator<Item = A>,
    {
        A::append_components(components, self);
    }
    fn empty() -> EmptyStorage<Self> {
        let storage = SoaStorage {
            types: HashSet::new(),
            anymap: Map::new(),
        };
        EmptyStorage { storage }
    }
    unsafe fn component_mut<T: Component>(&self) -> Option<&mut [T]> {
        self.anymap
            .get::<UnsafeStorage<T>>()
            .map(|vec| vec.get_mut_slice())
    }
    unsafe fn component<T: Component>(&self) -> Option<&[T]> {
        self.anymap
            .get::<UnsafeStorage<T>>()
            .map(|vec| vec.get_slice())
    }

    fn contains<C: Component>(&self) -> bool {
        self.anymap.contains::<UnsafeStorage<C>>()
    }
    fn types(&self) -> &HashSet<TypeId> {
        &self.types
    }
}
