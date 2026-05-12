self.addEventListener("activate", function (event) {
  if (self.registration.navigationPreload) {
    event.waitUntil(self.registration.navigationPreload.disable());
  }
});
