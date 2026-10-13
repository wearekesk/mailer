//! Modal dialogs and confirmation popups.

use leptos::prelude::*;

/// Reusable Flowbite-style confirmation modal.
#[component]
pub fn ConfirmDialog(
    /// Stable id for the modal `<div>` and the trigger button.
    #[prop(into)]
    id: String,
    /// `<form action="…">` URL — where the confirmed POST is sent.
    #[prop(into)]
    action: String,
    /// Inline trigger button label (e.g. `"Delete"`, `"Stop campaign"`).
    #[prop(into)]
    trigger_label: String,
    /// Body text of the confirmation prompt.
    #[prop(into)]
    message: String,
    /// Label on the destructive confirm button.
    #[prop(into, optional)]
    confirm_label: Option<String>,
    /// CSS classes applied to the trigger button.
    #[prop(into, optional)]
    trigger_class: Option<String>,
) -> impl IntoView {
    let confirm_label = confirm_label.unwrap_or_else(|| "Yes, I'm sure".into());
    let trigger_class = trigger_class
        .unwrap_or_else(|| "text-red-600 hover:underline text-sm".into());
    let modal_id = id.clone();
    let close_attr = format!("{modal_id}");

    view! {
        <button
            type="button"
            data-modal-target=modal_id.clone()
            data-modal-toggle=modal_id.clone()
            class=trigger_class
        >
            {trigger_label}
        </button>

        <div id=modal_id tabindex="-1" aria-hidden="true"
            class="hidden overflow-y-auto overflow-x-hidden fixed top-0 right-0 left-0 z-50 justify-center items-center w-full md:inset-0 h-modal md:h-full">
            <div class="relative p-4 w-full max-w-md h-full md:h-auto">
                <div class="relative p-4 text-center bg-white rounded-lg shadow dark:bg-gray-800 sm:p-5">
                    <button type="button" data-modal-toggle=close_attr.clone()
                        class="text-gray-400 absolute top-2.5 right-2.5 bg-transparent hover:bg-gray-200 hover:text-gray-900 rounded-lg text-sm p-1.5 ml-auto inline-flex items-center dark:hover:bg-gray-600 dark:hover:text-white">
                        <i class="fa-solid fa-xmark" aria-hidden="true"></i>
                        <span class="sr-only">"Close modal"</span>
                    </button>
                    <i class="fa-solid fa-trash-can text-gray-400 dark:text-gray-500 text-4xl mb-3.5" aria-hidden="true"></i>
                    <p class="mb-4 text-gray-500 dark:text-gray-300">{message}</p>
                    <form method="POST" action=action class="flex justify-center items-center space-x-4">
                        <button type="button" data-modal-toggle=close_attr
                            class="py-2 px-3 text-sm font-medium text-gray-500 bg-white rounded-lg border border-gray-200 hover:bg-gray-100 focus:ring-4 focus:outline-none focus:ring-primary-300 hover:text-gray-900 focus:z-10 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-500 dark:hover:text-white dark:hover:bg-gray-600 dark:focus:ring-gray-600">
                            "No, cancel"
                        </button>
                        <button type="submit"
                            class="py-2 px-3 text-sm font-medium text-center text-white bg-red-600 rounded-lg hover:bg-red-700 focus:ring-4 focus:outline-none focus:ring-red-300 dark:bg-red-500 dark:hover:bg-red-600 dark:focus:ring-red-900">
                            {confirm_label}
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}
