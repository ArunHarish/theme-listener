use super::ThemePublisher;

use crate::theme::Theme;
use objc2::{define_class, extern_methods, msg_send, AllocAnyThread, DefinedClass};
use objc2_app_kit::{NSAppearance, NSApplication};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions,
    NSObject, NSObjectNSKeyValueObserverRegistration, NSObjectProtocol, NSString,
};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;

use std::ffi::c_void;
use std::ptr;

struct ThemeObserverIvars {
    callback: Box<dyn Fn(Retained<NSString>)>,
}

impl ThemeObserver {
    fn new(callback: impl Fn(Retained<NSString>) + 'static) -> Retained<Self> {
        let observer = Self::alloc().set_ivars(ThemeObserverIvars {
            callback: Box::new(callback),
        });
        unsafe { msg_send![super(observer), init] }
    }

    extern_methods!(
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        pub fn observe_theme_change(
            &self,
            _key_path: &NSString,
            _object: &AnyObject,
            _change: &NSDictionary<NSKeyValueChangeKey, AnyObject>,
            _context: *mut c_void,
        ) -> ();
    );
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name="ThemeObserver"]
    #[ivars=ThemeObserverIvars]
    struct ThemeObserver;

    impl ThemeObserver {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn _observe_theme_change(&self, _key_path: &NSString, _object: &AnyObject, change: &NSDictionary<NSKeyValueChangeKey, NSAppearance>, _context: *mut c_void) {
            unsafe {
                let new_apperance: Retained<NSAppearance> = change.objectForKey(ns_string!("new")).unwrap();
                let value = new_apperance.name();
                (&self.ivars().callback)(value);
            }
        }
    }

    unsafe impl NSObjectProtocol for ThemeObserver {}
);

#[derive(Copy, Clone)]
pub struct KVOPublisher;

impl KVOPublisher {
    pub fn new() -> KVOPublisher {
        // Create a theme observer class
        KVOPublisher {}
    }
}

impl ThemePublisher<Retained<NSString>> for KVOPublisher {
    fn fetch(self) -> Result<Theme, Box<dyn std::error::Error>> {
        unsafe {
            let appearance = NSAppearance::currentAppearance().unwrap();
            let theme_value = appearance.name();
            let theme: Theme = self.to_theme(theme_value);
            Ok(theme)
        }
    }

    fn on_publish(self, callback: Box<dyn Fn(Theme) + Send + 'static>) {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApplication::sharedApplication(mtm);
        let observer = ThemeObserver::new(move |next_theme_value: Retained<NSString>| {
            let next_theme = self.to_theme(next_theme_value);
            callback(next_theme);
        });

        // Register app observer key path
        unsafe {
            app.addObserver_forKeyPath_options_context(
                &observer,
                ns_string!("effectiveAppearance"),
                NSKeyValueObservingOptions::New,
                ptr::null_mut(),
            );
        }
        app.run();
    }

    fn to_theme(self, theme_value: Retained<NSString>) -> Theme {
        unsafe {
            if theme_value.containsString(ns_string!("Dark")) {
                return Theme::DARK;
            }
        }
        return Theme::LIGHT;
    }
}
