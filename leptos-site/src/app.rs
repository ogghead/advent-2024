use components::{Route, Router, Routes};
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use leptos_meta::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use server_fn::codec::GetUrl;

#[cfg(feature = "ssr")]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone()/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/leptos_site.css"/>
        <Meta name="description" content="A website running its server-side as a WASI Component :D"/>

        <Title text="Welcome to Leptos X Spin!"/>

        <Router>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("/*any") view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // // Creates a reactive value to update the button
    // let (count, set_count) = signal(0);
    // let on_click = move |_| {
    //     set_count.update(|count| *count += 1);
    //     spawn_local(async move {
    //         save_count(count.get()).await.unwrap();
    //     });
    // };

    let wishlists = OnceResource::new(async {
        match get_wishlists().await {
            Ok(wishlists) => wishlists,
            Err(_) => vec![],
        }
    });

    let (name, set_name) = signal("Name".to_string());
    let name_element: NodeRef<html::Input> = NodeRef::new();

    let (wishlist_items, set_wishlist_items) = signal("".to_string());
    let wishlist_items_element: NodeRef<html::Input> = NodeRef::new();

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        // here, we'll extract the value from the input
        let name = name_element
            .get()
            // event handlers can only fire after the view
            // is mounted to the DOM, so the `NodeRef` will be `Some`
            .expect("<input> should be mounted")
            // `leptos::HtmlElement<html::Input>` implements `Deref`
            // to a `web_sys::HtmlInputElement`.
            // this means we can call`HtmlInputElement::value()`
            // to get the current value of the input
            .value();

        set_name.set(name.clone());

        // here, we'll extract the value from the input
        let wishlist = wishlist_items_element
            .get()
            // event handlers can only fire after the view
            // is mounted to the DOM, so the `NodeRef` will be `Some`
            .expect("<input> should be mounted")
            // `leptos::HtmlElement<html::Input>` implements `Deref`
            // to a `web_sys::HtmlInputElement`.
            // this means we can call`HtmlInputElement::value()`
            // to get the current value of the input
            .value();

        set_wishlist_items.set(wishlist.clone());

        spawn_local(async move {
            if let Err(e) = save_wishlist(Wishlist {
                name,
                items: wishlist.split(",").map(|item| item.to_owned()).collect(),
            })
            .await
            {
                println!("got error while saving wishlist! {e}");
            }
        });
    };

    view! {
      <h1>"Welcome to Leptos - served from Spin!"</h1>
      <form on:submit=on_submit>
          <input type="text"
            value=name
            node_ref=name_element
          />
          <input type="text"
            value=wishlist_items
            node_ref=wishlist_items_element
          />
          <input type="submit" value="Submit"/>
      </form>
      <Suspense
        fallback=move || view! { <p>"Loading..."</p> }
        >

        {
            move || {
                wishlists.get().map(|wishlists|
                    view! {
                        <ul>
                        <For each=move || wishlists.clone()
                           key=|wishlist| wishlist.name.clone()
                           children=move |wishlist| {
                              view! {
                                  <li>
                                      {wishlist.name}
                                  </li>
                              }
                           }
                        />
                    </ul>
                    }
                )
            }
        }
    </Suspense>

    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        if let Some(resp) = use_context::<leptos_wasi::response::ResponseOptions>() {
            resp.set_status(leptos_wasi::prelude::StatusCode::NOT_FOUND);
        }
    }

    view! { <h1>"Not Found"</h1> }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Wishlist {
    pub name: String,
    pub items: Vec<String>,
}

#[server(endpoint = "wishlists")]
pub async fn save_wishlist(wishlist: Wishlist) -> Result<(), ServerFnError<String>> {
    println!("saving wishlist!");
    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;

    store
        .set_json(wishlist.name.clone(), &wishlist)
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;

    Ok(())
}

#[server(endpoint = "wishlists", input = GetUrl)]
pub async fn get_wishlists() -> Result<Vec<Wishlist>, ServerFnError<String>> {
    println!("getting wishlists!");

    let store = spin_sdk::key_value::Store::open_default().map_err(|e| e.to_string())?;

    store
        .delete("leptos_site_count")
        .map_err(|e| e.to_string())?;

    let wishlist_keys = store.get_keys().map_err(|e| e.to_string())?;
    let wishlists =
        wishlist_keys
            .into_iter()
            .map(|name| {
                store.get_json::<Wishlist>(name).expect(
                "todo: graceful handling if a list is deleted in between getting list and now",
            ).expect("should have wishlist")
            })
            .collect();
    Ok(wishlists)
}
