use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use futures_util::StreamExt;
use indexmap::IndexMap;
pub use tapi_macro::{tapi, Tapi};

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}
impl Method {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RequestStructureBody {
    Query(&'static dyn Typed),
    Json(&'static dyn Typed),
    PlainText,
}
#[derive(Debug)]
pub struct RequestStructure {
    pub path: Option<&'static dyn Typed>,
    pub method: Method,
    pub body: Option<RequestStructureBody>,
}

impl RequestStructure {
    pub fn new(method: Method) -> Self {
        Self {
            path: None,
            method,
            body: None,
        }
    }
    pub fn merge_with(&mut self, req: RequestTapi) {
        match req {
            RequestTapi::Path(ty) => {
                self.path = Some(ty);
            }
            RequestTapi::Query(ty) => {
                self.body = Some(RequestStructureBody::Query(ty));
            }
            RequestTapi::Json(ty) => {
                self.body = Some(RequestStructureBody::Json(ty));
            }
            RequestTapi::None => {}
        }
    }
}
pub trait Endpoint {
    fn path(&self) -> &'static str;
    fn method(&self) -> Method;
    fn bind_to(&self, router: axum::Router) -> axum::Router;
    fn body(&self) -> RequestStructure;
    fn res(&self) -> ResponseTapi;
    fn tys(&self) -> Vec<&'static dyn Typed> {
        let mut tys = Vec::new();
        if let Some(path) = self.body().path {
            tys.push(path);
        }
        if let Some(body) = self.body().body {
            match body {
                RequestStructureBody::Query(ty) => {
                    tys.push(ty);
                }
                RequestStructureBody::Json(ty) => {
                    tys.push(ty);
                }
                RequestStructureBody::PlainText => {}
            }
        }
        tys.push(self.res().ty());
        tys
    }
    /// Generate a TypeScript client for this endpoint.
    ///
    /// The generated client will look something like this:
    /// ```ignore
    /// export const api = {
    ///     index: request<{}, string>("none", "GET", "/", "text"),
    ///     api: request<Person, string>("json", "GET", "/api", "json"),
    ///     api2AB: request<{}, string>("none", "GET", "/api2/:a/:b", "text"),
    ///     wow: sse<Msg>("/wow", "json"),
    ///     cool: request<Record<string, string>, Msg>("json", "GET", "/cool", "json"),
    /// };
    /// ```
    fn ts_client(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        match (self.body(), self.res()) {
            (RequestStructure { body: None, .. }, ResponseTapi::Sse(ty)) => {
                // TODO: handle non-json responses
                write!(s, "sse<{}>({:?}, \"json\")", ty.ts_name(), self.path(),).unwrap();
            }
            (RequestStructure { body, .. }, res) => {
                write!(
                    s,
                    "request<{}, {}>({:?}, {:?}, {:?}, {:?})",
                    match body {
                        Some(RequestStructureBody::Query(ty)) => ty.ts_name(),
                        Some(RequestStructureBody::Json(ty)) => ty.ts_name(),
                        // TODO: is this right?
                        Some(RequestStructureBody::PlainText) =>
                            "Record<string, never>".to_string(),
                        None => "Record<string, never>".to_string(),
                    },
                    res.ty().ts_name(),
                    match body {
                        Some(RequestStructureBody::Query(_)) => "query",
                        Some(RequestStructureBody::Json(_)) => "json",
                        Some(RequestStructureBody::PlainText) => "none",
                        None => "none",
                    },
                    self.method().as_str(),
                    self.path(),
                    match res {
                        ResponseTapi::PlainText => "text",
                        ResponseTapi::Bytes => "bytes",
                        ResponseTapi::Json(_) => "json",
                        ResponseTapi::Html => "html",
                        ResponseTapi::Sse(_) => "sse",
                        ResponseTapi::None => "none",
                    }
                )
                .unwrap();
            }
            x => todo!("unhandeled endpoint combination: {x:?}"),
        }
        s
    }
}
impl<'a, T> Endpoint for &'a T
where
    T: Endpoint,
{
    fn path(&self) -> &'static str {
        (*self).path()
    }
    fn method(&self) -> Method {
        (*self).method()
    }
    fn bind_to(&self, router: axum::Router) -> axum::Router {
        (*self).bind_to(router)
    }
    fn body(&self) -> RequestStructure {
        (*self).body()
    }
    fn res(&self) -> ResponseTapi {
        (*self).res()
    }
}

pub struct Endpoints<'a> {
    endpoints: Vec<&'a dyn Endpoint>,
    extra_tys: Vec<&'static dyn Typed>,
}
impl<'a> Endpoints<'a> {
    pub fn new(endpoints: impl IntoIterator<Item = &'a dyn Endpoint>) -> Self {
        Self {
            endpoints: endpoints.into_iter().collect(),
            extra_tys: Vec::new(),
        }
    }
    pub fn with_ty<T: Tapi + 'static>(mut self) -> Self {
        self.extra_tys.push(T::boxed());
        self
    }
    pub fn tys(&self) -> Vec<&'static dyn Typed> {
        let mut tys = self.extra_tys.clone();
        for endpoint in &self.endpoints {
            tys.extend(endpoint.tys());
        }
        tys.sort_by_key(|t| t.id());
        tys.dedup_by_key(|t| t.id());
        transitive_closure(tys)
    }
    pub fn ts_client(&self) -> String {
        let mut s = String::new();
        s.push_str(include_str!("../preamble.ts"));

        for ty in self.tys() {
            if let Some(decl) = ty.ts_decl() {
                s.push_str(&decl);
                s.push('\n');
            }
        }

        s.push_str("export const api = {\n");
        for endpoint in &self.endpoints {
            let name = heck::AsLowerCamelCase(endpoint.path()).to_string();
            let name = if name.is_empty() { "index" } else { &name };
            s.push_str(&format!("    {name}: {},\n", endpoint.ts_client()));
        }
        s.push_str("};\n");
        s
    }
}
impl<'a> IntoIterator for Endpoints<'a> {
    type Item = &'a dyn Endpoint;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.into_iter()
    }
}
impl<'s, 'a> IntoIterator for &'s Endpoints<'a> {
    type Item = &'a dyn Endpoint;
    type IntoIter = std::iter::Copied<std::slice::Iter<'s, &'a dyn Endpoint>>;
    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.iter().copied()
    }
}

pub trait RouterExt {
    fn tapi<E: Endpoint + ?Sized>(self, endpoint: &E) -> Self;
    fn tapis<'a>(self, endpoints: impl IntoIterator<Item = &'a dyn Endpoint>) -> Self
    where
        Self: Sized,
    {
        endpoints.into_iter().fold(self, Self::tapi)
    }
}

impl RouterExt for axum::Router {
    fn tapi<E: Endpoint + ?Sized>(self, endpoint: &E) -> Self {
        E::bind_to(endpoint, self)
    }
}

pub struct Sse<T, E = axum::BoxError>(futures_util::stream::BoxStream<'static, Result<T, E>>);
impl<T, E> Sse<T, E> {
    pub fn new<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<T, E>> + Send + 'static,
    {
        Self(stream.boxed())
    }
}
impl<T> axum::response::IntoResponse for Sse<T>
where
    T: serde::Serialize + 'static,
{
    fn into_response(self) -> axum::response::Response {
        let stream = self
            .0
            .map(|s| -> Result<axum::response::sse::Event, axum::BoxError> {
                let s = serde_json::to_string(&s?)?;
                Ok(axum::response::sse::Event::default().data(s))
            });
        axum::response::sse::Sse::new(stream).into_response()
    }
}

#[derive(Debug)]
pub enum RequestTapi {
    Path(&'static dyn Typed),
    Query(&'static dyn Typed),
    Json(&'static dyn Typed),
    None,
}
pub trait RequestTapiExtractor {
    fn extract_request() -> RequestTapi;
}
impl RequestTapiExtractor for () {
    fn extract_request() -> RequestTapi {
        RequestTapi::None
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::extract::Path<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Path(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::extract::Query<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Query(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> RequestTapiExtractor for axum::Json<T> {
    fn extract_request() -> RequestTapi {
        RequestTapi::Json(<T as Tapi>::boxed())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResponseTapi {
    // `text/plain; charset=utf-8`
    PlainText,
    // `application/octet-stream`
    Bytes,
    // `application/json`
    Json(&'static dyn Typed),
    // `text/html`
    Html,
    // `text/event-stream`
    Sse(&'static dyn Typed),
    None,
}
pub trait ResponseTapiExtractor {
    fn extract_response() -> ResponseTapi;
}
impl ResponseTapiExtractor for () {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::None
    }
}
impl ResponseTapiExtractor for String {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::PlainText
    }
}
impl ResponseTapiExtractor for Vec<u8> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Bytes
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for axum::Json<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Json(<T as Tapi>::boxed())
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for axum::response::Html<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Html
    }
}
impl<T: Tapi + 'static> ResponseTapiExtractor for Sse<T> {
    fn extract_response() -> ResponseTapi {
        ResponseTapi::Sse(<T as Tapi>::boxed())
    }
}

impl RequestTapi {
    pub fn ty(self) -> &'static dyn Typed {
        match self {
            Self::Path(ty) => ty,
            Self::Query(ty) => ty,
            Self::Json(ty) => ty,
            Self::None => <() as Tapi>::boxed(),
        }
    }
}
impl ResponseTapi {
    pub fn ty(self) -> &'static dyn Typed {
        match self {
            Self::PlainText => <String as Tapi>::boxed(),
            Self::Bytes => <Vec<u8> as Tapi>::boxed(),
            Self::Json(ty) => ty,
            Self::Html => <String as Tapi>::boxed(),
            Self::Sse(ty) => ty,
            Self::None => <() as Tapi>::boxed(),
        }
    }
}

pub trait Tapi {
    fn name() -> &'static str;
    fn id() -> std::any::TypeId;
    fn dependencies() -> Vec<&'static dyn Typed>;
    fn ts_name() -> String;
    fn zod_name() -> String;
    fn ts_decl() -> Option<String> {
        None
    }
    fn zod_decl() -> Option<String> {
        None
    }
    fn boxed() -> &'static dyn Typed
    where
        Self: Sized + 'static,
    {
        &TypedWrap::<Self>(PhantomData)
    }
}

pub trait Typed: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn id(&self) -> std::any::TypeId;
    fn dependencies(&self) -> Vec<&'static dyn Typed>;
    fn ts_name(&self) -> String;
    fn zod_name(&self) -> String;
    fn ts_decl(&self) -> Option<String>;
    fn zod_decl(&self) -> Option<String>;
}

impl<T: Tapi> Typed for TypedWrap<T> {
    fn name(&self) -> &'static str {
        <T as Tapi>::name()
    }
    fn id(&self) -> std::any::TypeId {
        <T as Tapi>::id()
    }
    fn dependencies(&self) -> Vec<&'static dyn Typed> {
        <T as Tapi>::dependencies()
    }
    fn ts_name(&self) -> String {
        <T as Tapi>::ts_name()
    }
    fn zod_name(&self) -> String {
        <T as Tapi>::zod_name()
    }
    fn ts_decl(&self) -> Option<String> {
        <T as Tapi>::ts_decl()
    }
    fn zod_decl(&self) -> Option<String> {
        <T as Tapi>::zod_decl()
    }
}

pub struct TypedWrap<T>(PhantomData<T>);
impl<T> TypedWrap<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}
impl<T> Clone for TypedWrap<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<T> std::fmt::Debug for TypedWrap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>()).finish()
    }
}

macro_rules! impl_typed {
    ($($ty:ty = $ts_name:literal,)*) => {
        $(
            impl Tapi for $ty {
                fn name() -> &'static str {
                    std::any::type_name::<$ty>()
                }
                fn id() -> std::any::TypeId {
                    std::any::TypeId::of::<$ty>()
                }
                fn dependencies() -> Vec<&'static dyn Typed> {
                    Vec::new()
                }
                fn ts_name() -> String {
                    $ts_name.to_string()
                }
                fn zod_name() -> String {
                    format!("z.{}()", $ts_name)
                }
            }
        )*
    };
}
macro_rules! impl_generic {
    ($($ty:ident = $ts_name:literal & $zod_name:literal,)*) => {
        $(
            impl<T: Tapi + 'static> Tapi for $ty<T> {
                fn name() -> &'static str {
                    std::any::type_name::<$ty<T>>()
                }
                fn id() -> std::any::TypeId {
                    std::any::TypeId::of::<$ty<T>>()
                }
                fn dependencies() -> Vec<&'static dyn Typed> {
                    if let Some(inner) = (&TypedWrap(PhantomData::<T>) as &dyn Any).downcast_ref::<&dyn Typed>() {
                        inner.dependencies()
                    } else {
                        Vec::new()
                    }
                }
                fn ts_name() -> String {
                    format!($ts_name, T::ts_name())
                }
                fn zod_name() -> String {
                    format!($zod_name, T::zod_name())
                }
            }
        )*
    };
}
impl_typed!(
    () = "unknown",
    String = "string",
    i8 = "number",
    i16 = "number",
    i32 = "number",
    i64 = "number",
    i128 = "number",
    u8 = "number",
    u16 = "number",
    u32 = "number",
    u64 = "number",
    u128 = "number",
    f32 = "number",
    f64 = "number",
    bool = "boolean",
    char = "string",
);
impl_generic!(
    Vec = "{}[]" & "z.array({})",
    Option = "{}?" & "z.optional({})",
    HashSet = "{}[]" & "z.array({})",
    Rc = "{}" & "{}",
    Arc = "{}" & "{}",
    Cell = "{}" & "{}",
    RefCell = "{}" & "{}",
);
impl<T: 'static + Tapi> Tapi for HashMap<String, T> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn dependencies() -> Vec<&'static dyn Typed> {
        T::dependencies()
    }
    fn ts_name() -> String {
        format!("Record<{}, {}>", String::ts_name(), T::ts_name())
    }
    fn zod_name() -> String {
        format!("z.record({}, {})", String::zod_name(), T::zod_name())
    }
}
impl<T: 'static + Tapi> Tapi for IndexMap<String, T> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn dependencies() -> Vec<&'static dyn Typed> {
        T::dependencies()
    }
    fn ts_name() -> String {
        format!("Record<{}, {}>", String::ts_name(), T::ts_name())
    }
    fn zod_name() -> String {
        format!("z.record({}, {})", String::zod_name(), T::zod_name())
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),*) => {
        impl<$($ty: 'static + Tapi),*> Tapi for ($($ty,)*) {
            fn name() -> &'static str {
                std::any::type_name::<Self>()
            }
            fn id() -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }
            fn dependencies() -> Vec<&'static dyn Typed> {
                let mut deps = Vec::new();
                $(
                    deps.extend(<$ty as Tapi>::dependencies());
                )*
                deps.sort_by_key(|t| t.id());
                deps.dedup_by_key(|t| t.id());
                deps
            }
            fn ts_name() -> String {
                format!(
                    "[{}]",
                    vec![$(<$ty as Tapi>::ts_name()),*].join(", ")
                )
            }
            fn zod_name() -> String {
                format!(
                    "z.tuple([{}])",
                    vec![$(<$ty as Tapi>::zod_name()),*].join(", ")
                )
            }
        }
    };
}
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);

impl Tapi for serde_json::Value {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn dependencies() -> Vec<&'static dyn Typed> {
        Vec::new()
    }
    fn ts_name() -> String {
        "unknown".to_string()
    }
    fn zod_name() -> String {
        "z.unknown()".to_string()
    }
}

fn transitive_closure(mut closure: Vec<&'static dyn Typed>) -> Vec<&'static dyn Typed> {
    let mut next = Vec::new();
    loop {
        for c in &closure {
            next.extend(c.dependencies().into_iter());
        }
        let mut done = true;
        for n in next.drain(..) {
            if closure.iter().all(|m| m.id() != n.id()) {
                done = false;
                closure.push(n);
            }
        }
        if done {
            break;
        }
    }
    closure
}
