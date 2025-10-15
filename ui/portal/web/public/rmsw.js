self.addEventListener("install", () => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    self.clients.claim().then(() => {
      return self.registration.unregister().then(() => {
        return self.clients.matchAll().then((clients) => {
          clients.forEach((client) => client.navigate(client.url));
        });
      });
    }),
  );
});
