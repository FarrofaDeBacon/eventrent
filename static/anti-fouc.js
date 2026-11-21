(function () {
  try {
    var lastTenant = localStorage.getItem("eventrent:brand:last") || "local";
    var raw = localStorage.getItem("eventrent:brand:" + lastTenant);
    if (!raw && lastTenant !== "local") {
      raw = localStorage.getItem("eventrent:brand:local");
    }
    if (raw) {
      var b = JSON.parse(raw);
      if (b.company_name) {
        document.title = b.company_name + " — EventRent";
      }
      if (b.primary_color) {
        document.documentElement.style.setProperty(
          "--primary",
          b.primary_color,
        );
        document.documentElement.style.setProperty("--ring", b.primary_color);
      }
      if (b.secondary_color) {
        document.documentElement.style.setProperty(
          "--secondary",
          b.secondary_color,
        );
      }
      if (b.accent_color) {
        document.documentElement.style.setProperty("--accent", b.accent_color);
      }
    }
  } catch {
    // Silently ignore storage or JSON parse errors during early anti-FOUC initialization
  }
})();
