// Safari runs this only when the person taps Play on TV. No account or feed data is read.
var ExtensionPreprocessingJS = {
  run: function (arguments) {
    arguments.completionFunction({url: window.location.href});
  },
  finalize: function (arguments) {
    if (!arguments.played || !/^(www\.|m\.)?youtube\.com$/.test(window.location.hostname)) return;
    document.querySelectorAll('video').forEach(function (video) { video.pause(); });
  }
};
