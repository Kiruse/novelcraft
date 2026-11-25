use dioxus::prelude::*;

use crate::auth;
use crate::comps::{Button, BtnVariant, Field, Screen, INPUT_CLASS};
use crate::screens::Route;

const LINK_CLASS: &str = "text-nc-label hover:underline cursor-pointer";

/// Login / account-creation screen. Registration requires an invite code;
/// the username is optional — when omitted, a random
/// `adjective.animal.NNN` name is chosen.
#[component]
pub fn Login() -> Element {
  let mut registering = use_signal(|| false);
  let mut email = use_signal(String::new);
  let mut password = use_signal(String::new);
  let mut invite_code = use_signal(String::new);
  let mut username = use_signal(String::new);
  let mut error: Signal<Option<String>> = use_signal(|| None);
  let mut busy = use_signal(|| false);

  let submit = move |_| {
    if *busy.read() {
      return;
    }
    let email_text = email.read().trim().to_string();
    let password_text = password.read().trim().to_string();
    let is_registering = *registering.read();
    if email_text.is_empty() || password_text.is_empty() {
      error.set(Some("Email and password are required.".to_string()));
      return;
    }
    let mut invite_text = String::new();
    let mut username_text = String::new();
    if is_registering {
      invite_text = invite_code.read().trim().to_string();
      if invite_text.is_empty() {
        error.set(Some("An invite code is required to create an account.".to_string()));
        return;
      }
      username_text = username.read().trim().to_string();
      if !username_text.is_empty() && !auth::valid_username(&username_text) {
        error.set(Some(
          "Usernames may only contain letters, digits, '.', '_' and '-' (1-30 characters)."
            .to_string(),
        ));
        return;
      }
    }

    let nav = navigator();
    spawn(async move {
      busy.set(true);
      error.set(None);
      let result = if is_registering {
        let username = (!username_text.is_empty()).then_some(username_text);
        auth::register(email_text, password_text, invite_text, username).await
      } else {
        auth::login(email_text, password_text).await
      };
      busy.set(false);
      match result {
        Ok(_) => {
          nav.push(Route::Home {});
        }
        Err(err) => error.set(Some(err.to_string())),
      }
    });
  };

  let switch_mode = move |_| {
    error.set(None);
    registering.toggle();
  };

  let title = if *registering.read() { "Create Account" } else { "Log In" };

  rsx! {
    Screen { title, navback: true, nouserbox: true,
      Field { label: "Email",
        input {
          class: INPUT_CLASS,
          r#type: "email",
          autocomplete: "email",
          placeholder: "you@example.com",
          value: "{email}",
          oninput: move |ev: FormEvent| email.set(ev.value()),
        }
      }
      Field { label: "Password",
        input {
          class: INPUT_CLASS,
          r#type: "password",
          autocomplete: if *registering.read() { "new-password" } else { "current-password" },
          value: "{password}",
          oninput: move |ev: FormEvent| password.set(ev.value()),
        }
      }
      { if *registering.read() {
          rsx! {
            Field { label: "Invite Code",
              input {
                class: INPUT_CLASS,
                placeholder: "Required",
                value: "{invite_code}",
                oninput: move |ev: FormEvent| invite_code.set(ev.value()),
              }
            }
            Field { label: "Username",
              hint: "Optional — leave blank and we'll pick a random name for you (e.g. clever.otter.123).",
              input {
                class: INPUT_CLASS,
                maxlength: 30,
                placeholder: "clever.otter.123",
                value: "{username}",
                oninput: move |ev: FormEvent| username.set(ev.value()),
              }
            }
          }
        } else { rsx! {} } }
      { error.read().as_ref().map(|msg| rsx! {
          div { class: "text-sm text-nc-dangerfg", { msg.clone() } }
      }) }
      Button {
        label: if *registering.read() { "Create Account" } else { "Log In" },
        variant: BtnVariant::Primary,
        disabled: *busy.read(),
        on_click: submit,
      }
      div { class: "text-center text-sm",
        { if *registering.read() {
            rsx! {
              "Already have an account? "
              a { class: LINK_CLASS, onclick: switch_mode, "Log in!" }
            }
          } else {
            rsx! {
              "New here? "
              a { class: LINK_CLASS, onclick: switch_mode, "Create new account!" }
            }
          } }
      }
    }
  }
}
