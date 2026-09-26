// Copy buttons, the install switcher and the mobile docs menu.

function copyText(button, text) {
  navigator.clipboard.writeText(text).then(() => {
    button.textContent = "copied";
    button.classList.add("done");
    setTimeout(() => {
      button.textContent = "copy";
      button.classList.remove("done");
    }, 1400);
  });
}

document.querySelectorAll("button.copy[data-copy]").forEach((button) => {
  button.addEventListener("click", () => copyText(button, button.dataset.copy));
});

// Every highlighted code block in the docs gets its own copy button.
document.querySelectorAll("div.highlight").forEach((block) => {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "copy";
  button.textContent = "copy";
  button.setAttribute("aria-label", "Copy code");
  const code = block.querySelector("code") || block;
  button.addEventListener("click", () => copyText(button, code.innerText.trim()));
  block.appendChild(button);
});

document.querySelectorAll(".install").forEach((install) => {
  const tabs = install.querySelectorAll(".install-tabs button");
  const panes = install.querySelectorAll(".install-pane");
  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      tabs.forEach((t) => t.classList.toggle("active", t === tab));
      panes.forEach((p) => p.classList.toggle("active", p.dataset.pane === tab.dataset.pane));
    });
  });
});

const toggle = document.querySelector(".sidebar-toggle");
if (toggle) {
  toggle.addEventListener("click", () => {
    const sidebar = document.getElementById("sidebar");
    const open = sidebar.classList.toggle("open");
    toggle.setAttribute("aria-expanded", String(open));
  });
}
