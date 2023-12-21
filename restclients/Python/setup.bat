curl  "http://localhost:8000/spectcl/attach/attach?type=file&source=run-0000-00.par"
curl  http://localhost:8000/spectcl/analyze/start
timeout /t 2 /nobreak
curl http://localhost:8000/spectcl/analyze/stop

