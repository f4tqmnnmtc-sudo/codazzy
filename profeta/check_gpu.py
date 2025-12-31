#!/usr/bin/env python3
import sys
import torch
import requests


def check_gpu():
    print(f"PyTorch: {torch.__version__}")
    print(f"CUDA disponible: {torch.cuda.is_available()}")

    if not torch.cuda.is_available():
        return False

    print(f"CUDA: {torch.version.cuda}")
    
    for i in range(torch.cuda.device_count()):
        props = torch.cuda.get_device_properties(i)
        mem_gb = props.total_memory / 1e9
        print(f"GPU {i}: {props.name} ({mem_gb:.1f} GB)")

    x = torch.randn(100, 100, device='cuda')
    y = torch.matmul(x, x)
    print(f"Test GPU OK, memoria usada: {torch.cuda.memory_allocated() / 1e6:.1f} MB")
    
    return True


def check_api():
    try:
        r = requests.get("http://localhost:8000/health", timeout=5)
        data = r.json()
        print(f"API: {data['status']}")
        print(f"Modelo: {data['model_name']}")
        
        r = requests.get("http://localhost:8000/gpu/status", timeout=5)
        gpu = r.json()
        if gpu.get('cuda_available'):
            print(f"GPU en API: {gpu.get('device_name')}")
        
        return data['status'] == 'healthy'
    except requests.exceptions.ConnectionError:
        print("API no disponible")
        return False


def test_predict():
    try:
        r = requests.post(
            "http://localhost:8000/predict",
            json={
                "data": {"values": [100, 105, 110, 108, 115, 120], "series_name": "test"},
                "prediction_length": 3
            },
            timeout=30
        )
        if r.status_code == 200:
            data = r.json()
            print(f"Predicción OK ({data['processing_time']:.2f}s)")
            return True
        print(f"Error: {r.status_code}")
        return False
    except Exception as e:
        print(f"Error: {e}")
        return False


if __name__ == "__main__":
    print("GPU")
    gpu_ok = check_gpu()
    
    print("\nAPI")
    api_ok = check_api()
    
    print("\n=== Predicción ===")
    pred_ok = test_predict()
    
    print("\nResumen")
    print(f"GPU: {'OK' if gpu_ok else 'KO'}")
    print(f"API: {'OK' if api_ok else 'KO'}")
    print(f"Predicción: {'OK' if pred_ok else 'KO'}")
    
    sys.exit(0 if all([gpu_ok, api_ok, pred_ok]) else 1)
