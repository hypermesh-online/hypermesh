package hypermesh

import (
	"context"
	"fmt"
)

// CaesarApi provides access to Caesar EVP endpoints.
type CaesarApi struct {
	http *httpClient
}

// Wallet returns wallet info.
func (a *CaesarApi) Wallet(ctx context.Context) (*CaesarWalletInfo, error) {
	var result CaesarWalletInfo
	if err := a.http.get(ctx, "/api/v1/caesar/wallet", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Balance returns the current balance.
func (a *CaesarApi) Balance(ctx context.Context) (*CaesarBalance, error) {
	var result CaesarBalance
	if err := a.http.get(ctx, "/api/v1/caesar/balance", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Transactions returns recent transactions. Pass 0 for no limit.
func (a *CaesarApi) Transactions(ctx context.Context, limit int) (*CaesarTransactionList, error) {
	path := "/api/v1/caesar/transactions"
	if limit > 0 {
		path = fmt.Sprintf("%s?limit=%d", path, limit)
	}
	var result CaesarTransactionList
	if err := a.http.get(ctx, path, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Rewards returns reward info.
func (a *CaesarApi) Rewards(ctx context.Context) (*CaesarRewardInfo, error) {
	var result CaesarRewardInfo
	if err := a.http.get(ctx, "/api/v1/caesar/rewards", &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// RoutePacket routes an EVP packet to the given destination.
func (a *CaesarApi) RoutePacket(ctx context.Context, destination string, amountGrams float64) (*CaesarRouteResult, error) {
	body := CaesarRouteRequest{
		Destination: destination,
		AmountGrams: amountGrams,
	}
	var result CaesarRouteResult
	if err := a.http.post(ctx, "/api/v1/caesar/route", body, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// GovernorParams returns the current governor parameters.
func (a *CaesarApi) GovernorParams(ctx context.Context) (*CaesarGovernorParams, error) {
	var result CaesarGovernorParams
	if err := a.http.get(ctx, "/api/v1/caesar/governor/params", &result); err != nil {
		return nil, err
	}
	return &result, nil
}
