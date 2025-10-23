// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting-started/welcome.html"><strong aria-hidden="true">1.</strong> Welcome to Fowl Engine</a></li><li class="chapter-item expanded "><a href="getting-started/joining-team.html"><strong aria-hidden="true">2.</strong> Joining a Team</a></li><li class="chapter-item expanded "><a href="getting-started/hud-and-menus.html"><strong aria-hidden="true">3.</strong> Understanding the Menus</a></li><li class="chapter-item expanded affix "><li class="part-title">Core Gameplay</li><li class="chapter-item expanded "><a href="gameplay/objectives.html"><strong aria-hidden="true">4.</strong> Objectives</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="gameplay/capturing-objectives.html"><strong aria-hidden="true">4.1.</strong> Capturing Objectives</a></li><li class="chapter-item expanded "><a href="gameplay/logistics.html"><strong aria-hidden="true">4.2.</strong> Logistics &amp; Supply</a></li></ol></li><li class="chapter-item expanded "><a href="gameplay/points-and-lives.html"><strong aria-hidden="true">5.</strong> Points and Lives System</a></li><li class="chapter-item expanded "><a href="gameplay/chat-commands.html"><strong aria-hidden="true">6.</strong> Chat Commands</a></li><li class="chapter-item expanded affix "><li class="part-title">F10 Menu Systems</li><li class="chapter-item expanded "><a href="f10-menu/overview.html"><strong aria-hidden="true">7.</strong> F10 Menu Overview</a></li><li class="chapter-item expanded "><a href="f10-menu/actions.html"><strong aria-hidden="true">8.</strong> Actions Menu</a></li><li class="chapter-item expanded "><a href="f10-menu/jtac.html"><strong aria-hidden="true">9.</strong> JTAC System</a></li><li class="chapter-item expanded "><a href="f10-menu/cargo.html"><strong aria-hidden="true">10.</strong> Cargo Operations</a></li><li class="chapter-item expanded "><a href="f10-menu/troops.html"><strong aria-hidden="true">11.</strong> Troop Transport</a></li><li class="chapter-item expanded "><a href="f10-menu/ewr.html"><strong aria-hidden="true">12.</strong> Early Warning Radar (EWR)</a></li><li class="chapter-item expanded affix "><li class="part-title">Advanced Topics</li><li class="chapter-item expanded "><a href="advanced/artillery.html"><strong aria-hidden="true">13.</strong> Artillery Missions</a></li><li class="chapter-item expanded "><a href="advanced/alcm.html"><strong aria-hidden="true">14.</strong> Air-to-Ground Cruise Missiles</a></li><li class="chapter-item expanded affix "><li class="part-title">Reference</li><li class="chapter-item expanded "><a href="reference/chat-commands.html"><strong aria-hidden="true">15.</strong> Complete Chat Command List</a></li><li class="chapter-item expanded "><a href="reference/action-types.html"><strong aria-hidden="true">16.</strong> Action Types Reference</a></li><li class="chapter-item expanded "><a href="reference/deployables.html"><strong aria-hidden="true">17.</strong> Deployable Units Reference</a></li><li class="chapter-item expanded "><a href="reference/faq.html"><strong aria-hidden="true">18.</strong> FAQ</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
