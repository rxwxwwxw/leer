use maud::{html, Markup, DOCTYPE};

pub fn base_layout(
    base_path: &str,
    title: &str,
    current_user: Option<&str>,
    header: Option<Markup>,
    section_header: Markup,
    content: Markup,
    sidebar: Option<Markup>,
    aftercontent: Option<Markup>,
    footer: Option<Markup>,
) -> Markup {
    let home_url = if base_path.is_empty() { "/" } else { base_path };
    let static_css = format!("{}/static/css/styles.css", base_path);
    let search_url = format!("{}/search", base_path);
    let authors_url = format!("{}/authors", base_path);
    let lists_url = format!("{}/lists", base_path);
    let logout_url = format!("{}/logout", base_path);
    let login_url = format!("{}/login", base_path);
    let register_url = format!("{}/register", base_path);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                meta name="color-scheme" content="light dark";
                title { (title) " | Fiction Archive" }
                link rel="stylesheet" href=(static_css);
            }
            body {
                header class="site-header" {
                    div class="header-inner" {
                        a class="site-logo" href=(home_url) { "Fiction Archive" }
                        nav class="site-nav" aria-label="Primary navigation" {
                            a class="nav-link" href=(search_url) { "Search" }
                            a class="nav-link" href=(authors_url) { "Authors" }
                            @if let Some(username) = current_user {
                                a class="nav-link" href=(lists_url) { "My Lists" }
                                span class="nav-user" { "Hello, " strong { (username) } }
                                form action=(logout_url) method="post" class="nav-form" {
                                    button type="submit" class="nav-button" { "Logout" }
                                }
                            } @else {
                                a class="nav-link" href=(login_url) { "Login" }
                                a class="nav-link nav-link-emphasis" href=(register_url) { "Register" }
                            }
                        }
                    }
                }
                main class="container" {
                    @if let Some(h) = header {
                        div class="page-header" { (h) }
                    }
                    (section_header)
                    div class=(if sidebar.is_some() { "main-grid" } else { "main-full" }) {
                        section class="content-body" {
                            (content)
                            @if let Some(ac) = aftercontent {
                                aside class="after-content" { (ac) }
                            }
                        }
                        @if let Some(sb) = sidebar {
                            aside class="sidebar-wrapper" { (sb) }
                        }
                    }
                }
                footer class="site-footer" {
                    div class="footer-inner" {
                        @if let Some(f) = footer { (f) }
                        p class="footer-text" { "Powered by Actix, Maud & SQLite Vector Search" }
                    }
                }
            }
        }
    }
}
