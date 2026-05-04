"use strict";

function storeValue(settingName, value) {
    try {
        localStorage.setItem(`clippy-lint-list-${settingName}`, value);
    } catch (e) { }
}

function loadValue(settingName) {
    return localStorage.getItem(`clippy-lint-list-${settingName}`);
}

function setTheme(theme, store) {
    let enableHighlight = false;
    let enableNight = false;
    let enableAyu = false;

    switch(theme) {
        case "ayu":
            enableAyu = true;
            break;
        case "coal":
        case "navy":
            enableNight = true;
            break;
        case "rust":
            enableHighlight = true;
            break;
        default:
            enableHighlight = true;
            theme = "light";
            break;
    }

    document.body.className = theme;

    document.getElementById("githubLightHighlight").disabled = enableNight || !enableHighlight;
    document.getElementById("githubDarkHighlight").disabled = !enableNight && !enableAyu;

    document.getElementById("styleHighlight").disabled = !enableHighlight;
    document.getElementById("styleNight").disabled = !enableNight;
    document.getElementById("styleAyu").disabled = !enableAyu;

    if (store) {
        storeValue("theme", theme);
    }
}

(function() {
    function generateSettingsButton() {
        const dropdown = document.createElement("div");
        dropdown.id = "settings-dropdown";
        dropdown.classList.add("dropdown");
        dropdown.innerHTML = `
    <button class="settings-icon" tabindex="-1"></button>
    <div class="settings-menu" tabindex="-1">
        <div class="setting-radio-name">Theme</div>
        <select id="theme-choice">
            <option value="ayu">Ayu</option>
            <option value="coal">Coal</option>
            <option value="light">Light</option>
            <option value="navy">Navy</option>
            <option value="rust">Rust</option>
        </select>
        <label>
            <input type="checkbox" id="disable-shortcuts"> {#+ #}
            <span>Disable keyboard shortcuts</span>
        </label>
    </div>`;
        document.body.insertBefore(dropdown, document.body.firstChild);
    }

    generateSettingsButton();

    // This file is loaded first. If so, we add the `js` class on the `<html>`
    // element.
    document.documentElement.classList.add("js");

    // loading the theme after the initial load
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
    const theme = loadValue("theme");
    if (prefersDark.matches && !theme) {
        setTheme("coal", false);
    } else {
        setTheme(theme, false);
    }

    const themeChoice = document.getElementById("theme-choice");

    themeChoice.value = loadValue("theme");
    document.getElementById("theme-choice").addEventListener("change", (e) => {
        setTheme(themeChoice.value, true);
    });
})();
