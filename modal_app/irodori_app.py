"""
Irodori-TTS on Modal — GPU-powered Japanese TTS with voice cloning.

Dev (logs visible in terminal, auto-reload on file change):
    cd modal_app && modal serve irodori_app.py

Deploy (stable URL):
    cd modal_app && modal deploy irodori_app.py

Test:
    curl -X POST https://<your-workspace>--irodori-tts-irodoritts-web.modal.run/synthesize \
         -H "Content-Type: application/json" \
         -d '{"text":"こんにちは"}' \
         --output test.wav
"""

import modal

app = modal.App("irodori-tts")

# ---------------------------------------------------------------------------
# Image: install dependencies and download model at build time
# ---------------------------------------------------------------------------
image = (
    modal.Image.debian_slim(python_version="3.11")
    .apt_install("git", "ffmpeg")
    # Install PyTorch with CUDA first
    .pip_install(
        "torch>=2.10.0",
        "torchaudio>=2.10.0",
        extra_index_url="https://download.pytorch.org/whl/cu128",
    )
    # Install Irodori-TTS dependencies and dacvae
    .pip_install(
        "transformers<5",
        "huggingface-hub>=0.34.0,<1.0",
        "peft>=0.18.0",
        "safetensors>=0.7.0",
        "soundfile>=0.12.0",
        "pyyaml>=6.0",
        "tqdm>=4.67.3",
        "numba",
        "sentencepiece",
        "dacvae@git+https://github.com/facebookresearch/dacvae",
        "torchcodec",
        "fastapi[standard]",
        "numpy",
    )
    # Clone Irodori-TTS and add to Python path via .pth file
    # (setuptools build fails due to flat-layout, so we skip install and use path directly)
    .run_commands(
        "git clone https://github.com/Aratako/Irodori-TTS.git /opt/irodori-tts",
        "echo '/opt/irodori-tts' > $(python -c 'import site; print(site.getsitepackages()[0])')/irodori-tts.pth",
    )
    # Pre-download HuggingFace models into the image so cold starts are faster
    .run_commands(
        "python -c \""
        "from huggingface_hub import hf_hub_download; "
        "hf_hub_download(repo_id='Aratako/Irodori-TTS-500M-v2', filename='model.safetensors'); "
        "hf_hub_download(repo_id='Aratako/Semantic-DACVAE-Japanese-32dim', filename='weights.pth')"
        "\""
    )
    # Reference WAV baked into the image.
    # Place your voice sample at modal_app/ref_wavs/default.wav before deploying.
    .add_local_file("ref_wavs/default.wav", "/root/ref.wav", copy=True)
)


# ---------------------------------------------------------------------------
# TTS Service
# ---------------------------------------------------------------------------
@app.cls(gpu="L40S", image=image, timeout=300)
class IrodoriTTS:
    @modal.enter()
    def load_model(self):
        from huggingface_hub import hf_hub_download
        from irodori_tts.inference_runtime import InferenceRuntime, RuntimeKey

        checkpoint_path = hf_hub_download(
            repo_id="Aratako/Irodori-TTS-500M-v2",
            filename="model.safetensors",
        )

        # Download codec weights file and pass the local path
        codec_path = hf_hub_download(
            repo_id="Aratako/Semantic-DACVAE-Japanese-32dim",
            filename="weights.pth",
        )

        key = RuntimeKey(
            checkpoint=checkpoint_path,
            model_device="cuda",
            codec_repo=codec_path,
            model_precision="fp32",
            codec_device="cpu",
            codec_precision="fp32",
        )
        self.runtime = InferenceRuntime.from_key(key)

    @modal.asgi_app()
    def web(self):
        import io

        import soundfile as sf
        from fastapi import FastAPI
        from fastapi.responses import Response
        from irodori_tts.inference_runtime import SamplingRequest
        from pydantic import BaseModel

        api = FastAPI()

        class SynthRequest(BaseModel):
            text: str
            num_steps: int = 40
            cfg_scale_text: float = 3.0
            cfg_scale_speaker: float = 5.0
            seed: int | None = None

        @api.get("/health")
        def health():
            return {"status": "ok"}

        @api.post("/synthesize")
        def synthesize(req: SynthRequest):
            import torchaudio

            sampling_req = SamplingRequest(
                text=req.text,
                ref_wav="/root/ref.wav",
                num_steps=req.num_steps,
                cfg_scale_text=req.cfg_scale_text,
                cfg_scale_speaker=req.cfg_scale_speaker,
                seed=req.seed,
            )
            result = self.runtime.synthesize(sampling_req)

            # save_wav と同じ方式: torchaudio.save → soundfile fallback
            buf = io.BytesIO()
            try:
                torchaudio.save(buf, result.audio.cpu(), result.sample_rate, format="wav")
            except RuntimeError:
                sf.write(buf, result.audio.cpu().squeeze(0).numpy(), result.sample_rate, format="WAV")
            buf.seek(0)
            return Response(content=buf.getvalue(), media_type="audio/wav")

        return api
