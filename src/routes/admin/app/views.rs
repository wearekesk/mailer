//! App views.

use leptos::prelude::*;

use crate::components::layout::{AdminDocument, AdminLayout};

#[component]
pub fn AdminApp() -> impl IntoView {
    view! {
        <AdminDocument title="Not found">
            <AdminLayout>
                <section class="bg-white dark:bg-gray-800 rounded-lg shadow-sm p-12 text-center">
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">"404"</h1>
                    <p class="text-gray-500 dark:text-gray-400 mb-6">"That page does not exist."</p>
                    <a href="/app/brands" class="text-blue-600 hover:underline text-sm font-medium">"Back to brands"</a>
                </section>
            </AdminLayout>
        </AdminDocument>
    }
}
