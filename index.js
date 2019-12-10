(function () {
  this.addEventListener('DOMContentLoaded', function(){
    // "this" is window when called with no receiver
    //
    // this is also not my favorite pattern, but when faced with the choice of
    // simplicity or pure functions, ill go with simplicity for a demo

    var setupInput,
        setupOutput,
        updateMain,
        updateHeader,
        document = this.document,
        RustyZ = this.RustyZ,
        form = document.getElementById( "form" ),
        input = document.getElementById( "player_input" ),
        content = document.getElementById("content"),
        left_header = document.querySelector("#header .left"),
        right_header = document.querySelector("#header .right"),
        main = document.getElementById("main");

    setupInput = function setupInput() {
      form.addEventListener( "submit", function () {
        var submitted = input.value;

        input.dataset.value = input.value;
        input.value = "";
        input.blur();

        content.append(submitted);
        var breakNode = document.createElement( "br" );
        content.appendChild(breakNode);
        content.appendChild(breakNode);

        setTimeout( function () {
          //we don't include the form because it is offset/over the content
          var height = (content.offsetHeight + header.offsetHeight);
          var container_height = main.offsetHeight;

          if( height > container_height ) {
            var diff = (height - container_height);
            main.style.paddingTop = -1 * diff + "px";
            header.style.top = diff + (header.offsetHeight - 2) + "px";
          }
        }, 50 );

      });

      main.addEventListener("click", function () {
        input.focus();
      });
    };

    setupOutput = function setupOutput() {
      RustyZ.subscribe(function(update) {
        switch (update.source) {
          case "main": updateMain(update); break;
          case "left":
          case "right": updateHeader(update); break;
        }
      });
    };

    updateMain = function updateMain(update) {
      let string = update.content.replace("\n", "<br/>");
      content.innerHTML += string;
    };

    updateHeader = function updateHeader(update) {
      let string = update.content;

      let el = (update.source == "left") ? left_header : right_header;
      el.innerHTML = string;
    };


    setupInput();
    setupOutput();

  });
}());
