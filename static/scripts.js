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
  document.body.addEventListener("msg:newMessage", function (evt) {
    if (Notification?.permission === "granted") {
      new Notification(evt.detail.message);
    }
  });
});
