//! Interfaccia di Refugium: galleria degli annunci, accesso e form di pubblicazione.

use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::{Announcement, AuthUser, Credentials};

use crate::api;

/// Vista corrente dell'app: galleria, inserimento oppure accesso/registrazione.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Gallery,
    Insert,
    Auth,
}

/// Modalità del form di autenticazione.
#[derive(Debug, Clone, Copy, PartialEq)]
enum AuthMode {
    Login,
    Register,
}

#[component]
pub fn App() -> impl IntoView {
    // Elenco degli annunci: risorsa locale (CSR), ricaricabile dopo l'inserimento.
    let announcements = LocalResource::new(|| api::list_announcements());
    let (page, set_page) = signal(Page::Gallery);
    let (user, set_user) = signal(None::<AuthUser>);

    // All'avvio ripristina la sessione se il browser ha un JWT valido.
    spawn_local(async move {
        match api::me().await {
            Ok(current) => set_user.set(Some(current)),
            Err(_) => api::clear_token(),
        }
    });

    // «Inserisci» apre il form se siamo autenticati, altrimenti la pagina di accesso.
    let open_insert = move |_: web_sys::MouseEvent| {
        set_page.set(if user.get().is_some() {
            Page::Insert
        } else {
            Page::Auth
        });
    };
    let logout = move |_: web_sys::MouseEvent| {
        api::clear_token();
        set_user.set(None);
        set_page.set(Page::Gallery);
    };

    view! {
        <div class="app">
            <header class="header">
                <div class="brand">
                    <h1 on:click=move |_| set_page.set(Page::Gallery)>"🐕 Refugium"</h1>
                    <p class="subtitle">"Annunci di cani in cerca di una casa"</p>
                </div>
                <nav class="nav">
                    {move || match page.get() {
                        Page::Gallery => None,
                        _ => Some(
                            view! {
                                <button
                                    class="nav-button nav-ghost"
                                    on:click=move |_| set_page.set(Page::Gallery)
                                >
                                    "← Torna alla galleria"
                                </button>
                            }
                        ),
                    }}
                    <button class="nav-button" on:click=open_insert>
                        "+ Inserisci"
                    </button>
                    {move || match user.get() {
                        Some(current) => view! {
                            <span class="session-email">{current.email.clone()}</span>
                            <button class="nav-button nav-ghost" on:click=logout>"Esci"</button>
                        }
                        .into_any(),
                        None => view! {
                            <button
                                class="nav-button nav-ghost"
                                on:click=move |_| set_page.set(Page::Auth)
                            >
                                "Accedi"
                            </button>
                        }
                        .into_any(),
                    }}
                </nav>
            </header>

            <main class="main">
                {move || match page.get() {
                    Page::Gallery => view! { <GalleryView announcements /> }.into_any(),
                    Page::Insert => view! {
                        <AnnouncementForm announcements set_page set_user />
                    }
                    .into_any(),
                    Page::Auth => view! { <AuthForm set_user set_page /> }.into_any(),
                }}
            </main>

            <footer class="footer">
                <p>"Refugium — backend Axum + frontend Leptos"</p>
            </footer>
        </div>
    }
}

#[component]
fn AnnouncementForm(
    announcements: LocalResource<Result<Vec<Announcement>, String>>,
    set_page: WriteSignal<Page>,
    set_user: WriteSignal<Option<AuthUser>>,
) -> impl IntoView {
    let (dog_name, set_dog_name) = signal(String::new());
    let (breed, set_breed) = signal(String::new());
    let (age, set_age) = signal(String::new());
    let (city, set_city) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (contact, set_contact) = signal(String::new());
    let (image, set_image) = signal(None::<web_sys::File>);

    let (submitting, set_submitting) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (success, set_success) = signal(None::<String>);

    // Errore di validazione per ogni campo (mostrato sotto il campo stesso).
    let (dog_name_error, set_dog_name_error) = signal(None::<String>);
    let (breed_error, set_breed_error) = signal(None::<String>);
    let (age_error, set_age_error) = signal(None::<String>);
    let (city_error, set_city_error) = signal(None::<String>);
    let (description_error, set_description_error) = signal(None::<String>);
    let (contact_error, set_contact_error) = signal(None::<String>);
    let (image_error, set_image_error) = signal(None::<String>);

    // Aggiorna il valore e, se il campo era già segnato in errore, rivalida
    // a ogni battitura per far sparire l'errore appena il valore diventa valido.
    let on_dog_name_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if dog_name_error.get().is_some() {
            set_dog_name_error
                .set(validate_required(&value, MAX_DOG_NAME, "Il nome del cane").err());
        }
        set_dog_name.set(value);
    };
    let on_breed_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if breed_error.get().is_some() {
            set_breed_error.set(validate_optional(&value, MAX_BREED, "La razza").err());
        }
        set_breed.set(value);
    };
    let on_age_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if age_error.get().is_some() {
            set_age_error.set(validate_optional(&value, MAX_AGE, "L'età").err());
        }
        set_age.set(value);
    };
    let on_city_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if city_error.get().is_some() {
            set_city_error.set(validate_optional(&value, MAX_CITY, "La città").err());
        }
        set_city.set(value);
    };
    let on_description_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if description_error.get().is_some() {
            set_description_error
                .set(validate_required(&value, MAX_DESCRIPTION, "La descrizione").err());
        }
        set_description.set(value);
    };
    let on_contact_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if contact_error.get().is_some() {
            set_contact_error.set(validate_contact(&value).err());
        }
        set_contact.set(value);
    };

    // Valida la foto appena scelta: formato e dimensione, senza aspettare il submit.
    let on_file_change = move |event: web_sys::Event| {
        let input = event_target::<web_sys::HtmlInputElement>(&event);
        let file = input.files().and_then(|files| files.get(0));
        match file {
            None => {
                set_image.set(None);
                set_image_error.set(None);
            }
            Some(file) => match validate_image(&file) {
                Ok(()) => {
                    set_image.set(Some(file));
                    set_image_error.set(None);
                }
                Err(message) => {
                    // Foto non valida: scartata subito e campo svuotato,
                    // così riscegliendo lo stesso file l'evento parte di nuovo.
                    set_image.set(None);
                    set_image_error.set(Some(message));
                    input.set_value("");
                }
            },
        }
        set_error.set(None);
    };

    let on_submit = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        set_error.set(None);
        set_success.set(None);

        let name_value = dog_name.get();
        let breed_value = breed.get();
        let age_value = age.get();
        let city_value = city.get();
        let description_value = description.get();
        let contact_value = contact.get();

        // Valida tutti i campi: ogni errore appare sotto il proprio campo.
        set_dog_name_error
            .set(validate_required(&name_value, MAX_DOG_NAME, "Il nome del cane").err());
        set_breed_error.set(validate_optional(&breed_value, MAX_BREED, "La razza").err());
        set_age_error.set(validate_optional(&age_value, MAX_AGE, "L'età").err());
        set_city_error.set(validate_optional(&city_value, MAX_CITY, "La città").err());
        set_description_error
            .set(validate_required(&description_value, MAX_DESCRIPTION, "La descrizione").err());
        set_contact_error.set(validate_contact(&contact_value).err());
        if image.get().is_none() {
            set_image_error.set(Some("Scegli una foto del cane.".to_string()));
        }

        let all_valid = dog_name_error.get().is_none()
            && breed_error.get().is_none()
            && age_error.get().is_none()
            && city_error.get().is_none()
            && description_error.get().is_none()
            && contact_error.get().is_none()
            && image_error.get().is_none();
        if !all_valid {
            return;
        }

        let Some(file) = image.get() else {
            return;
        };

        // Valori ripuliti prima dell'invio (il backend rifà i propri controlli).
        let data = api::NewAnnouncement {
            dog_name: name_value.trim().to_string(),
            breed: optional_field(breed_value),
            age: optional_field(age_value),
            city: optional_field(city_value),
            description: description_value.trim().to_string(),
            contact: contact_value.trim().to_string(),
        };

        set_submitting.set(true);
        let announcements = announcements.clone();
        spawn_local(async move {
            match api::create_announcement(&data, file).await {
                Ok(created) => {
                    set_success.set(Some(format!(
                        "Annuncio di {} pubblicato! 🎉",
                        created.dog_name
                    )));
                    set_dog_name.set(String::new());
                    set_breed.set(String::new());
                    set_age.set(String::new());
                    set_city.set(String::new());
                    set_description.set(String::new());
                    set_contact.set(String::new());
                    set_image.set(None);
                    announcements.refetch();
                    // Torna alla galleria, dove la nuova foto è già visibile.
                    set_page.set(Page::Gallery);
                }
                Err(message) => {
                    if message.contains("401") {
                        // Sessione scaduta: riporta l'utente all'accesso.
                        api::clear_token();
                        set_user.set(None);
                        set_page.set(Page::Auth);
                    } else {
                        set_error.set(Some(message));
                    }
                }
            }
            set_submitting.set(false);
        });
    };

    view! {
        <section class="card form-card">
            <h2>"Pubblica un annuncio"</h2>
            <form on:submit=on_submit novalidate=true>
                <div class="form-grid">
                    <label class="field" class:invalid=move || dog_name_error.get().is_some()>
                        <span>"Nome del cane *"</span>
                        <input
                            type="text"
                            placeholder="Es. Rocky"
                            maxlength=MAX_DOG_NAME
                            prop:value=move || dog_name.get()
                            on:input=on_dog_name_input
                        />
                        {move || {
                            dog_name_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field" class:invalid=move || breed_error.get().is_some()>
                        <span>"Razza"</span>
                        <input
                            type="text"
                            placeholder="Es. Meticcio"
                            maxlength=MAX_BREED
                            prop:value=move || breed.get()
                            on:input=on_breed_input
                        />
                        {move || {
                            breed_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field" class:invalid=move || age_error.get().is_some()>
                        <span>"Età"</span>
                        <input
                            type="text"
                            placeholder="Es. 3 anni"
                            maxlength=MAX_AGE
                            prop:value=move || age.get()
                            on:input=on_age_input
                        />
                        {move || {
                            age_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field" class:invalid=move || city_error.get().is_some()>
                        <span>"Città"</span>
                        <input
                            type="text"
                            placeholder="Es. Roma"
                            maxlength=MAX_CITY
                            prop:value=move || city.get()
                            on:input=on_city_input
                        />
                        {move || {
                            city_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field" class:invalid=move || contact_error.get().is_some()>
                        <span>"Contatto *"</span>
                        <input
                            type="text"
                            placeholder="Telefono o email"
                            maxlength=MAX_CONTACT
                            prop:value=move || contact.get()
                            on:input=on_contact_input
                        />
                        {move || {
                            contact_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field" class:invalid=move || image_error.get().is_some()>
                        <span>"Foto del cane *"</span>
                        <input
                            type="file"
                            accept="image/png,image/jpeg,image/webp,image/gif"
                            on:change=on_file_change
                        />
                        {move || {
                            image_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field field-full" class:invalid=move || description_error.get().is_some()>
                        <span>"Descrizione *"</span>
                        <textarea
                            rows="3"
                            maxlength=MAX_DESCRIPTION
                            placeholder="Raccontaci di lui: carattere, abitudini, perché cerca una casa…"
                            prop:value=move || description.get()
                            on:input=on_description_input
                        ></textarea>
                        {move || {
                            description_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                        <span
                            class="field-counter"
                            class:warn=move || {
                                description.get().chars().count() > MAX_DESCRIPTION - 100
                            }
                        >
                            {move || format!("{} / {MAX_DESCRIPTION}", description.get().chars().count())}
                        </span>
                    </label>
                    {move || {
                        image.get().map(|file| {
                            view! {
                                <p class="field-hint field-full">
                                    "Foto selezionata: " {file.name()} " (" {human_size(file.size())} ")"
                                </p>
                            }
                            .into_any()
                        })
                    }}
                </div>

                {move || {
                    error.get().map(|message| {
                        view! { <p class="alert alert-error">{message}</p> }.into_any()
                    })
                }}
                {move || {
                    success.get().map(|message| {
                        view! { <p class="alert alert-success">{message}</p> }.into_any()
                    })
                }}

                <button type="submit" class="button" disabled=move || submitting.get()>
                    {move || if submitting.get() { "Pubblico…" } else { "Pubblica annuncio" }}
                </button>
            </form>
        </section>
    }
}

/// Form di accesso/registrazione: al successo salva il JWT e apre il form annunci.
#[component]
fn AuthForm(set_user: WriteSignal<Option<AuthUser>>, set_page: WriteSignal<Page>) -> impl IntoView {
    let (mode, set_mode) = signal(AuthMode::Login);
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (email_error, set_email_error) = signal(None::<String>);
    let (password_error, set_password_error) = signal(None::<String>);
    let (error, set_error) = signal(None::<String>);
    let (submitting, set_submitting) = signal(false);

    let switch_mode = move |next: AuthMode| {
        set_mode.set(next);
        set_email_error.set(None);
        set_password_error.set(None);
        set_error.set(None);
    };

    let on_email_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if email_error.get().is_some() {
            set_email_error.set(validate_email(&value).err());
        }
        set_email.set(value);
        set_error.set(None);
    };

    let on_password_input = move |event: web_sys::Event| {
        let value = event_target_value(&event);
        if password_error.get().is_some() {
            set_password_error.set(validate_password(&value).err());
        }
        set_password.set(value);
        set_error.set(None);
    };

    let on_submit = move |event: web_sys::SubmitEvent| {
        event.prevent_default();

        let email_value = email.get();
        let password_value = password.get();
        set_email_error.set(validate_email(&email_value).err());
        set_password_error.set(validate_password(&password_value).err());
        if email_error.get().is_some() || password_error.get().is_some() {
            return;
        }

        let credentials = Credentials {
            email: email_value.trim().to_lowercase(),
            password: password_value,
        };
        let registering = mode.get() == AuthMode::Register;

        set_submitting.set(true);
        spawn_local(async move {
            let result = if registering {
                api::register(&credentials).await
            } else {
                api::login(&credentials).await
            };
            match result {
                Ok(response) => {
                    api::set_token(&response.token);
                    set_user.set(Some(response.user));
                    // Dopo l'accesso si arriva dritti al form dell'annuncio.
                    set_page.set(Page::Insert);
                }
                Err(message) => set_error.set(Some(message)),
            }
            set_submitting.set(false);
        });
    };

    view! {
        <section class="card form-card auth-card">
            <div class="auth-tabs">
                <button
                    type="button"
                    class:active=move || mode.get() == AuthMode::Login
                    on:click=move |_| switch_mode(AuthMode::Login)
                >
                    "Accedi"
                </button>
                <button
                    type="button"
                    class:active=move || mode.get() == AuthMode::Register
                    on:click=move |_| switch_mode(AuthMode::Register)
                >
                    "Registrati"
                </button>
            </div>

            <h2>
                {move || {
                    if mode.get() == AuthMode::Register {
                        "Crea un account"
                    } else {
                        "Bentornato"
                    }
                }}
            </h2>
            <p class="auth-hint">
                "Serve un account per pubblicare annunci; la galleria resta sempre pubblica."
            </p>

            <form on:submit=on_submit novalidate=true>
                <div class="form-grid">
                    <label class="field field-full" class:invalid=move || email_error.get().is_some()>
                        <span>"Email *"</span>
                        <input
                            type="email"
                            placeholder="nome@esempio.it"
                            autocomplete="email"
                            prop:value=move || email.get()
                            on:input=on_email_input
                        />
                        {move || {
                            email_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                    <label class="field field-full" class:invalid=move || password_error.get().is_some()>
                        <span>
                            "Password *"
                            {move || {
                                if mode.get() == AuthMode::Register {
                                    " (minimo 8 caratteri)"
                                } else {
                                    ""
                                }
                            }}
                        </span>
                        <input
                            type="password"
                            placeholder="La tua password"
                            autocomplete="current-password"
                            prop:value=move || password.get()
                            on:input=on_password_input
                        />
                        {move || {
                            password_error.get().map(|message| {
                                view! { <span class="field-error">{message}</span> }.into_any()
                            })
                        }}
                    </label>
                </div>

                {move || {
                    error.get().map(|message| {
                        view! { <p class="alert alert-error">{message}</p> }.into_any()
                    })
                }}

                <button type="submit" class="button" disabled=move || submitting.get()>
                    {move || {
                        if submitting.get() {
                            "Attendere…"
                        } else if mode.get() == AuthMode::Register {
                            "Registrati e inserisci"
                        } else {
                            "Accedi e inserisci"
                        }
                    }}
                </button>
            </form>
        </section>
    }
}

/// Prima schermata: galleria delle foto degli annunci.
/// Cliccando una foto si apre il dettaglio dell'annuncio in una modale.
#[component]
fn GalleryView(announcements: LocalResource<Result<Vec<Announcement>, String>>) -> impl IntoView {
    let (selected, set_selected) = signal(Option::<Announcement>::None);

    view! {
        <Suspense fallback=|| {
            view! { <p class="loading">"Caricamento galleria…"</p> }
        }>
            {move || {
                let announcements = announcements.clone();
                Suspend::new(async move {
                    match announcements.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="empty">
                                "Ancora nessun annuncio: usa «Inserisci» nel menu per pubblicare il primo! 🐶"
                            </p>
                        }
                        .into_any(),
                        Ok(list) => view! {
                            <div class="gallery-grid">
                                {list
                                    .into_iter()
                                    .map(|announcement| {
                                        let for_click = announcement.clone();
                                        view! {
                                            <button
                                                class="gallery-item"
                                                title=format!("Dettagli di {}", announcement.dog_name)
                                                on:click=move |_| set_selected.set(Some(for_click.clone()))
                                            >
                                                <img
                                                    class="gallery-photo"
                                                    src=announcement.image_url.clone()
                                                    alt=format!("Foto di {}", announcement.dog_name)
                                                />
                                                <span class="gallery-name">
                                                    {announcement.dog_name.clone()}
                                                </span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any(),
                        Err(message) => view! {
                            <p class="alert alert-error">
                                "Impossibile caricare gli annunci: " {message}
                            </p>
                        }
                        .into_any(),
                    }
                })
            }}
        </Suspense>

        {move || {
            selected.get().map(|announcement| {
                view! { <DetailModal announcement set_selected /> }.into_any()
            })
        }}
    }
}

/// Modale con il dettaglio completo dell'annuncio.
/// Chiude cliccando sulla ✕, sullo sfondo o tornando alla galleria.
#[component]
fn DetailModal(
    announcement: Announcement,
    set_selected: WriteSignal<Option<Announcement>>,
) -> impl IntoView {
    let published_on = announcement.created_at.chars().take(10).collect::<String>();
    let image_alt = format!("Foto di {}", announcement.dog_name);
    let close = move |_| set_selected.set(None);

    view! {
        <div class="modal-backdrop" on:click=close.clone()>
            <div
                class="modal-card card dog-card"
                on:click=move |event: web_sys::MouseEvent| event.stop_propagation()
            >
                <button class="modal-close" title="Chiudi" on:click=close>"✕"</button>
                <img class="dog-photo" src=announcement.image_url.clone() alt=image_alt />
                <div class="dog-body">
                    <h3 class="dog-name">{announcement.dog_name.clone()}</h3>
                    <div class="tags">
                        {announcement
                            .breed
                            .clone()
                            .map(|breed| view! { <span class="tag">{breed}</span> })}
                        {announcement
                            .age
                            .clone()
                            .map(|age| view! { <span class="tag">{age}</span> })}
                        {announcement
                            .city
                            .clone()
                            .map(|city| view! { <span class="tag">{city}</span> })}
                    </div>
                    <p class="dog-description">{announcement.description.clone()}</p>
                    <p class="dog-contact">"Contatto: " {announcement.contact.clone()}</p>
                    <p class="dog-date">"Pubblicato il " {published_on}</p>
                </div>
            </div>
        </div>
    }
}

/// Un campo opzionale compilato diventa `Some`, vuoto/spazi `None`.
fn optional_field(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Dimensione leggibile per il nome del file selezionato.
fn human_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else {
        format!("{:.1} MB", bytes / (1024.0 * 1024.0))
    }
}

// --- Validazione dei campi (i limiti immagine rispecchiano il backend) ------

const MAX_DOG_NAME: usize = 50;
const MAX_BREED: usize = 50;
const MAX_AGE: usize = 30;
const MAX_CITY: usize = 50;
const MAX_CONTACT: usize = 100;
const MAX_DESCRIPTION: usize = 1000;

/// Dimensione massima della foto: 10 MB, come accettata dal backend.
const MAX_IMAGE_SIZE: f64 = 10.0 * 1024.0 * 1024.0;

/// Formati immagine accettati dal backend.
const ACCEPTED_IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

/// Campo obbligatorio: non vuoto (dopo i trim) e sotto il limite di caratteri.
fn validate_required(value: &str, max: usize, label: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{label} è obbligatorio."))
    } else if trimmed.chars().count() > max {
        Err(format!("{label} non può superare i {max} caratteri."))
    } else {
        Ok(())
    }
}

/// Campo opzionale: se compilato, deve rispettare il limite di caratteri.
fn validate_optional(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.trim().chars().count() > max {
        Err(format!("{label} non può superare i {max} caratteri."))
    } else {
        Ok(())
    }
}

/// Contatto: obbligatorio; se contiene "@", deve essere un'email plausibile.
fn validate_contact(value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Il contatto è obbligatorio.".to_string());
    }
    if trimmed.chars().count() > MAX_CONTACT {
        return Err(format!(
            "Il contatto non può superare i {MAX_CONTACT} caratteri."
        ));
    }
    if trimmed.contains('@') && !is_valid_email(trimmed) {
        return Err("L'indirizzo email non è valido (es. nome@esempio.it).".to_string());
    }
    Ok(())
}

/// Controlli minimi sull'email: una sola "@", parte locale e dominio non vuoti,
/// niente punti agli estremi né consecutivi, dominio con almeno un punto.
fn is_valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if parts.next().is_some() || local.is_empty() || domain.is_empty() {
        return false;
    }
    // Niente punti agli estremi né raddoppiati in nessuna delle due parti.
    let clean = |part: &str| !part.starts_with('.') && !part.ends_with('.') && !part.contains("..");
    clean(local) && domain.contains('.') && clean(domain)
}

/// Email dell'account: stesse regole del backend.
fn validate_email(value: &str) -> Result<(), String> {
    if !is_valid_email(value.trim()) {
        return Err("Inserisci un indirizzo email valido (es. nome@esempio.it).".to_string());
    }
    Ok(())
}

/// Password dell'account: tra 8 e 128 caratteri, come richiesto dal backend.
fn validate_password(value: &str) -> Result<(), String> {
    let length = value.chars().count();
    if length < 8 {
        return Err("La password deve contenere almeno 8 caratteri.".to_string());
    }
    if length > 128 {
        return Err("La password non può superare i 128 caratteri.".to_string());
    }
    Ok(())
}

/// Immagine: formato e dimensione devono essere accettati dal backend.
fn validate_image(file: &web_sys::File) -> Result<(), String> {
    let mime = file.type_();
    if mime.is_empty() || !ACCEPTED_IMAGE_TYPES.contains(&mime.as_str()) {
        return Err("Formato non supportato: usa un'immagine png, jpeg, webp o gif.".to_string());
    }
    if file.size() > MAX_IMAGE_SIZE {
        return Err(format!(
            "La foto è troppo grande ({}): il limite è 10 MB.",
            human_size(file.size())
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_rejects_empty_and_over_limit() {
        assert!(validate_required("   ", MAX_DOG_NAME, "Il nome del cane").is_err());
        assert!(validate_required("", MAX_DOG_NAME, "Il nome del cane").is_err());

        let too_long = "x".repeat(MAX_DOG_NAME + 1);
        assert!(validate_required(&too_long, MAX_DOG_NAME, "Il nome del cane").is_err());

        assert!(validate_required("Rocky", MAX_DOG_NAME, "Il nome del cane").is_ok());
        // I caratteri accentati contano come un carattere (non come byte).
        assert!(
            validate_required(&"è".repeat(MAX_DOG_NAME), MAX_DOG_NAME, "Il nome del cane").is_ok()
        );
    }

    #[test]
    fn optional_accepts_empty_but_checks_limit() {
        assert!(validate_optional("", MAX_BREED, "La razza").is_ok());
        assert!(validate_optional("Meticcio", MAX_BREED, "La razza").is_ok());

        let too_long = "y".repeat(MAX_AGE + 1);
        assert!(validate_optional(&too_long, MAX_AGE, "L'età").is_err());
    }

    #[test]
    fn contact_requires_value_and_valid_email() {
        assert!(validate_contact("").is_err());
        assert!(validate_contact("   ").is_err());

        let too_long = "z".repeat(MAX_CONTACT + 1);
        assert!(validate_contact(&too_long).is_err());

        // Con la "@" deve essere un'email plausibile.
        assert!(validate_contact("nome@esempio.it").is_ok());
        assert!(validate_contact("+39 348 1234567").is_ok());
        assert!(validate_contact("senza-chiocciola").is_ok());
        assert!(validate_contact("nome@").is_err());
        assert!(validate_contact("@esempio.it").is_err());
        assert!(validate_contact("nome@doppio..it").is_err());
        assert!(validate_contact("nome@esempio").is_err());
        assert!(validate_contact("a@b@c.it").is_err());
    }

    #[test]
    fn email_rules() {
        assert!(is_valid_email("mario.rossi@example.com"));
        assert!(is_valid_email("a@b.co"));
        assert!(!is_valid_email("nessuna-chiocciola"));
        assert!(!is_valid_email("due@chiocciole@x.it"));
        assert!(!is_valid_email(".punto@prima.it"));
        assert!(!is_valid_email("punto@finale.it."));
        assert!(!is_valid_email("doppio@punto..it"));
    }

    #[test]
    fn auth_email_validation() {
        assert!(validate_email(" mario.rossi@example.com ").is_ok());
        assert!(validate_email("nome@example").is_err());
        assert!(validate_email("senza-chiocciola").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn auth_password_validation() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("password lunghissima").is_ok());
        assert!(validate_password("1234567").is_err());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }
}
