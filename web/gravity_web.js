// Gravity Engine in the browser: plays the samples the game makes (sound
// effects, music) through WebAudio (see src/audio/out.rs), keeps the
// player's files (settings, levels, the ghost…) in the browser's local
// storage (see src/storage.rs), and takes the loading message away once the
// game has started.
(function () {
    "use strict";
    var ctx = null;
    var voices = {};
    var next = 1;

    function context() {
        if (!ctx) {
            var AudioContext = window.AudioContext || window.webkitAudioContext;
            if (!AudioContext) {
                return null;
            }
            ctx = new AudioContext();
        }
        return ctx;
    }

    // Browsers keep audio off until the page is used.
    function wake() {
        var c = context();
        if (c && c.state === "suspended") {
            c.resume();
        }
    }
    ["keydown", "mousedown", "pointerdown", "touchstart"].forEach(function (e) {
        window.addEventListener(e, wake, true);
    });

    // Storage: the result of the last query, until the game fetches it.
    var pending = null;
    var decoder = new TextDecoder();
    var encoder = new TextEncoder();

    function text(ptr, len) {
        return decoder.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    }

    function register_plugin(importObject) {
        // what = 0: the value under the key; 1: the keys starting with it.
        importObject.env.ge_storage_query = function (what, ptr, len) {
            var key = text(ptr, len);
            var value = null;
            try {
                if (what === 0) {
                    value = localStorage.getItem(key);
                } else {
                    var keys = [];
                    for (var i = 0; i < localStorage.length; i++) {
                        var k = localStorage.key(i);
                        if (k.indexOf(key) === 0) {
                            keys.push(k);
                        }
                    }
                    value = keys.join("\n");
                }
            } catch (e) {
                value = null;
            }
            if (value === null) {
                pending = null;
                return -1;
            }
            pending = encoder.encode(value);
            return pending.length;
        };
        importObject.env.ge_storage_fetch = function (ptr) {
            if (pending) {
                new Uint8Array(wasm_memory.buffer, ptr, pending.length).set(pending);
            }
            pending = null;
        };
        importObject.env.ge_storage_set = function (kptr, klen, vptr, vlen) {
            try {
                localStorage.setItem(text(kptr, klen), text(vptr, vlen));
                return 1;
            } catch (e) {
                return 0;
            }
        };

        // `len` interleaved samples of `channels` at `rate` Hz from wasm memory.
        importObject.env.ge_audio_play = function (ptr, len, channels, rate, volume, looped) {
            var c = context();
            if (!c || len === 0 || channels === 0) {
                return 0;
            }
            var data = new Float32Array(wasm_memory.buffer, ptr, len);
            var frames = Math.floor(len / channels);
            var buffer = c.createBuffer(channels, frames, rate);
            for (var ch = 0; ch < channels; ch++) {
                var out = buffer.getChannelData(ch);
                for (var i = 0; i < frames; i++) {
                    out[i] = data[i * channels + ch];
                }
            }
            var source = c.createBufferSource();
            source.buffer = buffer;
            source.loop = looped !== 0;
            var gain = c.createGain();
            gain.gain.value = volume;
            source.connect(gain);
            gain.connect(c.destination);
            var id = next++;
            voices[id] = { source: source, gain: gain };
            source.onended = function () {
                delete voices[id];
            };
            source.start();
            return id;
        };
        importObject.env.ge_audio_volume = function (id, volume) {
            var v = voices[id];
            if (v) {
                v.gain.gain.value = volume;
            }
        };
        importObject.env.ge_audio_stop = function (id) {
            var v = voices[id];
            if (v) {
                try {
                    v.source.stop();
                } catch (e) {
                    // Already stopped.
                }
                delete voices[id];
            }
        };
    }

    function on_init() {
        var loading = document.getElementById("loading");
        if (loading) {
            loading.remove();
        }
    }

    miniquad_add_plugin({ register_plugin: register_plugin, on_init: on_init, name: "gravity_web", version: 1 });
})();
