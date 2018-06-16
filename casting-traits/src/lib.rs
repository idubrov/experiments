#![feature(raw)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::any::TypeId;
use std::raw::TraitObject;

/// Helper trait to allow upcasting trait objects back to `&Plugin`, so they can be downcasted again.
pub trait PluginBase {
    fn as_plugin(&self) -> &(Plugin + 'static);
}

impl<T: Plugin> PluginBase for T {
    fn as_plugin(&self) -> &Plugin {
        self
    }
}

/// The core piece of the trait downcasting
pub trait Plugin: PluginBase + 'static {
    fn __downcast_ref(&self, target: TypeId) -> Option<TraitObject> {
        // Default implementation to downcast to the concrete type only
        if target == ::std::any::TypeId::of::<Self>() {
            Some(::std::raw::TraitObject {
                data: self as *const _ as *mut (),
                vtable: std::ptr::null_mut(),
            })
        } else {
            None
        }
    }
}

impl Plugin {
    /// Downcast to a given type, which could be either the concrete type or a trait type.
    pub fn downcast_ref<T: ?Sized + 'static>(&self) -> Option<&T> {
        unsafe {
            if let Some(obj) = self.__downcast_ref(TypeId::of::<T>()) {
                Some(*(&obj as *const TraitObject as *const &T))
            } else {
                None
            }
        }
    }
}

/// Helper macro to declare which traits given type should be downcastable to.
#[macro_export]
macro_rules! declare_interfaces (
    ( $typ: ident, [ $( $iface: ident ),* ]) => {
        impl Plugin for $typ {
            fn __downcast_ref(&self, target: ::std::any::TypeId) -> Option<::std::raw::TraitObject> {
                unsafe {
                    $(
                    if target == ::std::any::TypeId::of::<$iface>() {
                        return Some(::std::mem::transmute(self as &$iface));
                    }
                    )*
                }
                if target == ::std::any::TypeId::of::<$typ>() {
                    Some(::std::raw::TraitObject {
                        data: self as *const _ as *mut (),
                        vtable: std::ptr::null_mut(),
                    })
                } else {
                    None
                }
            }
        }
    }
);


// Example code


// Interfaces

/// Generate a greeting message for the given name.
trait Greeter: Plugin {
    fn greet(&self, name: &str) -> String;
}

/// This is a formal version, which uses a first name and the a name.
trait FormalGreeter: Plugin {
    fn greet_formal(&self, first_name: &str, last_name: &str) -> String;
}


// Implementations

/// Simple greeter
pub struct SimpleGreeter(String);

impl Greeter for SimpleGreeter {
    fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.0, name)
    }
}

impl FormalGreeter for SimpleGreeter {
    fn greet_formal(&self, first_name: &str, last_name: &str) -> String {
        format!("{}, {} {}!", self.0, first_name, last_name)
    }
}

declare_interfaces!(SimpleGreeter, [Greeter, FormalGreeter]);

// Counting greeter

pub struct CountingGreeter(AtomicUsize);

impl FormalGreeter for CountingGreeter {
    fn greet_formal(&self, first_name: &str, last_name: &str) -> String {
        self.0.fetch_add(1, Ordering::Relaxed);
        format!("Greetings, {} {}.", last_name, first_name)
    }
}

declare_interfaces!(CountingGreeter, [FormalGreeter]);

/// Polymorphic function

pub fn rsvp(first_name: &str, last_name: &str, plugin: &Plugin) -> String {
    if let Some(gt) = plugin.downcast_ref::<Greeter>() {
        return gt.greet(first_name);
    }
    if let Some(gt) = plugin.downcast_ref::<FormalGreeter>() {
        return gt.greet_formal(first_name, last_name);
    }
    "Hello?".to_string()
}

#[test]
fn test() {
    let simple: &Plugin = &SimpleGreeter("Hi".to_string()) as &Plugin;
    let formal: &Plugin = &CountingGreeter(Default::default()) as &Plugin;
    assert_eq!("Hi, Andrew!", rsvp("Andrew", "Baker", simple));
    assert_eq!("Greetings, Baker Andrew.", rsvp("Andrew", "Baker", formal));

    // cast back to the concrete type
    let simple = simple.downcast_ref::<SimpleGreeter>().unwrap();
    assert_eq!(simple.0, "Hi");

    // cast back to the concrete type
    let formal = formal.downcast_ref::<CountingGreeter>().unwrap();
    assert_eq!(1, formal.0.load(Ordering::Relaxed));
}

#[test]
fn cross_casting() {
    let simple: &Plugin = &SimpleGreeter("Hi".to_string()) as &Plugin;
    let simple = simple.downcast_ref::<Greeter>().unwrap();
    assert_eq!("Hi, Andrew!", simple.greet("Andrew"));

    // cross-casting via upcasting back to &Plugin
    let simple = simple.as_plugin().downcast_ref::<FormalGreeter>().unwrap();
    assert_eq!("Hi, Andrew Baker!", simple.greet_formal("Andrew", "Baker"));
}