
#include "stats.h"
#include <cuda_runtime.h>
#include <cmath>

__device__ double warp_reduce_double(double val) {
    for (int offset = 16; offset > 0; offset >>= 1) {
        val += __shfl_down_sync(0xFFFFFFFF, val, offset);
    }
    return val;
}

template<int BLOCK_SIZE, int ELEMENTS_PER_THREAD>
__global__ void stats_kernel(
    const float* __restrict__ input,
    double* __restrict__ sum_out,
    double* __restrict__ sum_sq_out,
    int n
) {
    __shared__ double s_sum[BLOCK_SIZE/32];
    __shared__ double s_sum_sq[BLOCK_SIZE/32];

    int tid = threadIdx.x;
    int gid = blockIdx.x * BLOCK_SIZE * ELEMENTS_PER_THREAD + tid;

    double local_sum = 0, local_sum_sq = 0;

    #pragma unroll
    for (int i = 0; i < ELEMENTS_PER_THREAD; i++) {
        int idx = gid + i * BLOCK_SIZE;
        if (idx < n) {
            double val = input[idx];
            local_sum += val;
            local_sum_sq += val * val;
        }
    }

    local_sum = warp_reduce_double(local_sum);
    local_sum_sq = warp_reduce_double(local_sum_sq);

    int lane = tid%32;
    int warpId = tid/32;

    if (lane == 0) {
        s_sum[warpId] = local_sum;
        s_sum_sq[warpId] = local_sum_sq;
    }

    __syncthreads();

    if (tid < BLOCK_SIZE/32) {
        local_sum = s_sum[tid];
        local_sum_sq = s_sum_sq[tid];
    }
    else {
        local_sum = 0;
        local_sum_sq = 0;
    }

    if (warpId == 0) {
        local_sum = warp_reduce_double(local_sum);
        local_sum_sq = warp_reduce_double(local_sum_sq);
        if (lane == 0) {
            atomicAdd(sum_out, local_sum);
            atomicAdd(sum_sq_out, local_sum_sq);
        }
    }
}

extern "C" int compute_stats(const float* data, int n, StatsResult* result) {
    const int BLOCK_SIZE = 256;
    const int ELEMENTS_PER_THREAD = 8;
    const int num_blocks = (n + BLOCK_SIZE * ELEMENTS_PER_THREAD - 1) / (BLOCK_SIZE * ELEMENTS_PER_THREAD); // ceiling division

    float* d_data;
    double* d_sum;
    double* d_sum_sq;

    if (cudaMalloc(&d_data, n*sizeof(float)) != cudaSuccess) return -1;
    if (cudaMalloc(&d_sum, sizeof(double)) != cudaSuccess) { 
        cudaFree(d_data); 
        return -1; 
    }
    if (cudaMalloc(&d_sum_sq, sizeof(double)) != cudaSuccess) { 
        cudaFree(d_data); 
        cudaFree(d_sum); 
        return -1; 
    }

    cudaMemcpy(d_data, data, n*sizeof(float), cudaMemcpyHostToDevice);
    cudaMemset(d_sum, 0, sizeof(double));
    cudaMemset(d_sum_sq, 0, sizeof(double));

    stats_kernel<BLOCK_SIZE, ELEMENTS_PER_THREAD><<<num_blocks, BLOCK_SIZE>>>(
        d_data, d_sum, d_sum_sq, n
    );

    double h_sum, h_sum_sq;

    cudaMemcpy(&h_sum, d_sum, sizeof(double), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_sum_sq, d_sum_sq, sizeof(double), cudaMemcpyDeviceToHost);
    
    result->mean = h_sum / n;
    result->variance = h_sum_sq/n - (result->mean * result->mean);
    result->stddev = sqrt(result->variance);

    cudaFree(d_data);
    cudaFree(d_sum);
    cudaFree(d_sum_sq);

    return 0;
}

extern "C" int compute_sharpe(const float* returns, int n, float risk_free_rate, double* sharpe) {
    StatsResult stats;
    int ret = compute_stats(returns, n, &stats);
    if (ret) return ret;

    if (stats.stddev < 1e-10) {
        *sharpe = 0;
    }
    else {
        *sharpe = (stats.mean - risk_free_rate) / stats.stddev;
    }

    return 0;
}
