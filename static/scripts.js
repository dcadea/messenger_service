function askNotificationPermission() {
  if (!("Notification" in window)) {
    console.log("This browser does not support notifications.");
    return;
  }
  Notification.requestPermission().then((permission) => {
    console.log(permission === "granted" ? "none" : "block");
  });
}

window.addEventListener("load", function () {
  document.body.addEventListener("htmx:sseMessage", function (evt) {
    if (document.hasFocus()) {
      // don't push notifications if current tab is active
      return;
    }

    if (Notification?.permission === "granted") {
      if (evt.detail.type.startsWith("newMessage")) {
        new Notification("You've got new message");
      }

      if (evt.detail.type.startsWith("newFriend")) {
        new Notification("You've got a new friend");
      }
    }
  });
});
