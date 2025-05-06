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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="index.html">Greeting</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Dango DEX</li><li class="chapter-item expanded "><a href="dex/passive-liquidity/index.html"><strong aria-hidden="true">1.</strong> Passive liquidity</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Audits</li><li class="chapter-item expanded "><a href="audits/index.html"><strong aria-hidden="true">2.</strong> List of audits completed</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Unorganized notes</li><li class="chapter-item expanded "><a href="notes/bounded-values.html"><strong aria-hidden="true">3.</strong> Bounded values</a></li><li class="chapter-item expanded "><a href="notes/exports.html"><strong aria-hidden="true">4.</strong> Exports</a></li><li class="chapter-item expanded "><a href="notes/extension-traits.html"><strong aria-hidden="true">5.</strong> Extension traits</a></li><li class="chapter-item expanded "><a href="notes/gas.html"><strong aria-hidden="true">6.</strong> Gas</a></li><li class="chapter-item expanded "><a href="notes/generate-dependency-graph.html"><strong aria-hidden="true">7.</strong> Generating dependency graph</a></li><li class="chapter-item expanded "><a href="notes/indexed-map.html"><strong aria-hidden="true">8.</strong> Indexed map</a></li><li class="chapter-item expanded "><a href="notes/liquidity-provision.html"><strong aria-hidden="true">9.</strong> Liquidity provision</a></li><li class="chapter-item expanded "><a href="notes/margin-account-health.html"><strong aria-hidden="true">10.</strong> Margin account: health</a></li><li class="chapter-item expanded "><a href="notes/math.html"><strong aria-hidden="true">11.</strong> Math</a></li><li class="chapter-item expanded "><a href="notes/nonces.html"><strong aria-hidden="true">12.</strong> Nonces and unordered transactions</a></li><li class="chapter-item expanded "><a href="notes/transaction-lifecycle.html"><strong aria-hidden="true">13.</strong> Transaction lifecycle</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Networks</li><li class="chapter-item expanded "><a href="networks/index.html"><strong aria-hidden="true">14.</strong> Overview</a></li><li class="chapter-item expanded "><a href="networks/dev-1.html"><strong aria-hidden="true">15.</strong> dev-1</a></li><li class="chapter-item expanded "><a href="networks/dev-2.html"><strong aria-hidden="true">16.</strong> dev-2</a></li><li class="chapter-item expanded "><a href="networks/dev-3.html"><strong aria-hidden="true">17.</strong> dev-3</a></li><li class="chapter-item expanded "><a href="networks/dev-4.html"><strong aria-hidden="true">18.</strong> dev-4</a></li><li class="chapter-item expanded "><a href="networks/dev-5.html"><strong aria-hidden="true">19.</strong> dev-5</a></li></ol>';
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
