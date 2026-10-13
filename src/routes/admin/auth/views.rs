//! Admin auth handlers — login, logout, two-factor, password reset.
//!
//! Page components are co-located here and use the Flowbite blocks rendered
//! by [`AuthDocument`] and [`AuthCard`].

use leptos::prelude::*;
use serde::Deserialize;

use crate::components::layout::{AuthCard, AuthDocument};

// ── Page components (Flowbite blocks) ────────────────────────────────────────

#[component]
pub fn LoginPage(#[prop(optional)] error: Option<String>, #[prop(optional)] info: Option<String>) -> impl IntoView {
    view! {
        <AuthDocument title="Sign in">
            <AuthCard
                heading="Sign in"
                subtitle="Enter your credentials to access your email dashboard."
            >
                {error.map(|m| view! {
                    <div class="p-3 mb-5 text-xs font-medium text-red-800 dark:text-red-300 rounded-lg bg-red-50 dark:bg-red-950/40 border border-red-200 dark:border-red-900/50 flex items-center gap-2" role="alert">
                        <i class="fa-solid fa-circle-exclamation text-red-600 dark:text-red-400"></i>
                        <span>{m}</span>
                    </div>
                })}
                {info.map(|m| view! {
                    <div class="p-3 mb-5 text-xs font-medium text-green-800 dark:text-green-300 rounded-lg bg-green-50 dark:bg-green-950/40 border border-green-200 dark:border-green-900/50 flex items-center gap-2" role="alert">
                        <i class="fa-solid fa-circle-check text-green-600 dark:text-green-400"></i>
                        <span>{m}</span>
                    </div>
                })}
                <form class="space-y-5" method="POST" action="/login">
                    <div>
                        <label for="email" class="block mb-1.5 text-xs font-semibold uppercase tracking-wider text-gray-700 dark:text-gray-300">
                            "Email address"
                        </label>
                        <div class="relative">
                            <div class="absolute inset-y-0 start-0 flex items-center ps-3.5 pointer-events-none text-gray-400 dark:text-gray-500">
                                <i class="fa-regular fa-envelope text-xs"></i>
                            </div>
                            <input
                                type="email" name="email" id="email" autocomplete="username" required
                                placeholder="name@company.com"
                                class="bg-gray-50/50 dark:bg-gray-800/60 border border-gray-300 dark:border-gray-700 text-gray-900 dark:text-white text-sm rounded-xl focus:ring-2 focus:ring-gray-900 dark:focus:ring-white focus:border-transparent block w-full ps-10 p-2.5 transition-all"
                            />
                        </div>
                    </div>
                    <div>
                        <div class="flex items-center justify-between mb-1.5">
                            <label for="password" class="block text-xs font-semibold uppercase tracking-wider text-gray-700 dark:text-gray-300">
                                "Password"
                            </label>
                            <a href="/forgot-password" class="text-xs font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors">
                                "Forgot password?"
                            </a>
                        </div>
                        <div class="relative">
                            <div class="absolute inset-y-0 start-0 flex items-center ps-3.5 pointer-events-none text-gray-400 dark:text-gray-500">
                                <i class="fa-solid fa-lock text-xs"></i>
                            </div>
                            <input
                                type="password" name="password" id="password" autocomplete="current-password" required
                                placeholder="••••••••"
                                class="bg-gray-50/50 dark:bg-gray-800/60 border border-gray-300 dark:border-gray-700 text-gray-900 dark:text-white text-sm rounded-xl focus:ring-2 focus:ring-gray-900 dark:focus:ring-white focus:border-transparent block w-full ps-10 p-2.5 transition-all"
                            />
                        </div>
                    </div>
                    <button
                        type="submit"
                        class="w-full text-white bg-gray-900 hover:bg-black dark:bg-white dark:text-gray-950 dark:hover:bg-gray-100 font-semibold rounded-xl text-sm px-5 py-3 text-center shadow-sm hover:shadow transition-all flex items-center justify-center gap-2"
                    >
                        <span>"Sign in"</span>
                        <i class="fa-solid fa-arrow-right text-xs"></i>
                    </button>
                </form>
            </AuthCard>
        </AuthDocument>
    }
}

#[component]
pub fn TwoFactorPage(#[prop(optional)] error: Option<String>) -> impl IntoView {
    view! {
        <AuthDocument title="Two-factor authentication">
            <AuthCard heading="Two-factor authentication">
                <p class="text-sm text-gray-500 dark:text-gray-400">"Enter the 6-digit code from your authenticator app."</p>
                {error.map(|m| view! {
                    <div class="p-3 mb-2 text-sm text-red-800 rounded-lg bg-red-50 border border-red-200" role="alert">
                        {m}
                    </div>
                })}
                <form class="space-y-4 md:space-y-6" method="POST" action="/2fa">
                    <div>
                        <label for="otp_code" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Authentication code"</label>
                        <input
                            type="text" name="otp_code" id="otp_code"
                            autocomplete="one-time-code" inputmode="numeric"
                            pattern="[0-9]{6}" maxlength="6" required
                            placeholder="000000"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5 tracking-[0.5em] text-center font-mono"
                        />
                    </div>
                    <button
                        type="submit"
                        class="w-full text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5"
                    >
                        "Verify and sign in"
                    </button>
                </form>
            </AuthCard>
        </AuthDocument>
    }
}

#[component]
pub fn ForgotPasswordPage(#[prop(optional)] info: Option<String>) -> impl IntoView {
    view! {
        <AuthDocument title="Forgot password">
            <AuthCard heading="Forgot password">
                {info.map(|m| view! {
                    <div class="p-3 mb-2 text-sm text-green-800 rounded-lg bg-green-50 border border-green-200" role="alert">
                        {m}
                    </div>
                })}
                <p class="text-sm text-gray-500 dark:text-gray-400">"Enter your account email and we'll send you a password-reset link."</p>
                <form class="space-y-4 md:space-y-6" method="POST" action="/forgot-password">
                    <div>
                        <label for="email" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">"Email"</label>
                        <input
                            type="email" name="email" id="email" required
                            placeholder="name@company.com"
                            class="bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-white sm:text-sm rounded-lg focus:ring-blue-600 focus:border-blue-600 block w-full p-2.5"
                        />
                    </div>
                    <button
                        type="submit"
                        class="w-full text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5"
                    >
                        "Send reset link"
                    </button>
                    <p class="text-sm text-gray-500 dark:text-gray-400 text-center">
                        <a href="/login" class="font-medium text-blue-600 hover:underline">"Back to sign in"</a>
                    </p>
                </form>
            </AuthCard>
        </AuthDocument>
    }
}

// ── Poem handlers ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    /// Error code from a prior failed attempt (1 = missing fields, 2 = bad creds)
    pub e: Option<u8>,
    /// Info code (1 = password reset email sent)
    pub i: Option<u8>,
}

