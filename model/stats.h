#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    double mean;
    double variance;
    double stddev;
} StatsResult;

int compute_stats(const float* data, int n, StatsResult* result);
int compute_sharpe(const float* returns, int n, float risk_free_rate, double* sharpe);

#ifdef __cplusplus
}
#endif