use winrt::windows::data::xml::dom::*;
use winrt::windows::ui::notifications::*;
use winrt::*;

use std::cell::RefCell;
use std::str;

thread_local! {
    static TOAST_NOTIFIER : RefCell<winrt::ComPtr<ToastNotifier>> =
        RefCell::new(ToastNotificationManager::create_toast_notifier_with_id(
            // Use PowerShell's App ID to circumvent the need to register one.
            &FastHString::new("{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe")
        ).unwrap());

    static PREVIOUS_TOAST: RefCell<Option<ComPtr<ToastNotification>>> = RefCell::new(None);
}

pub fn toast_notification(content: &str) {
    TOAST_NOTIFIER.with(|toast_notifier| {
        let toast_notifier = &*toast_notifier.borrow();

        PREVIOUS_TOAST.with(|prev_toast| {
            let prev_toast = &mut *prev_toast.borrow_mut();

            // If there's any previous toast, hide it right away.
            let should_hide_previous = if let &mut Some(ref toast) = prev_toast {
                unsafe {
                    toast_notifier.hide(toast).ok();
                }
                true
            } else {
                false
            };

            if should_hide_previous {
                *prev_toast = None;
            }

            unsafe {
                // Get a toast XML template
                let toast_xml =
                    ToastNotificationManager::get_template_content(ToastTemplateType::ToastText02)
                        .unwrap();

                // Fill in the text elements
                let toast_text_elements = toast_xml
                    .get_elements_by_tag_name(&FastHString::new("text"))
                    .unwrap();

                toast_text_elements
                    .item(0)
                    .unwrap()
                    .append_child(
                        &*toast_xml
                            .create_text_node(&FastHString::new("input-replay"))
                            .unwrap()
                            .query_interface::<IXmlNode>()
                            .unwrap(),
                    )
                    .unwrap();
                toast_text_elements
                    .item(1)
                    .unwrap()
                    .append_child(
                        &*toast_xml
                            .create_text_node(&FastHString::new(content))
                            .unwrap()
                            .query_interface::<IXmlNode>()
                            .unwrap(),
                    )
                    .unwrap();

                // Create the toast and attach event listeners
                let toast = ToastNotification::create_toast_notification(&*toast_xml).unwrap();

                // Show the toast
                (*toast_notifier).show(&*toast).unwrap();

                // Save it for next time, so we can hide it quickly
                *prev_toast = Some(toast);
            }
        });
    });
}
