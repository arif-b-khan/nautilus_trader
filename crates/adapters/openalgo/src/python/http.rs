// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use nautilus_core::python::to_pyvalue_err;
use pyo3::prelude::*;

use crate::http::client::OpenAlgoHttpClient;

#[pymethods]
impl OpenAlgoHttpClient {
    #[new]
    #[pyo3(signature = (api_key=None, host="http://127.0.0.1:5000", version="v1", ws_url="ws://127.0.0.1:8765", timeout_secs=None, proxy_url=None))]
    fn py_new(
        api_key: Option<String>,
        host: &str,
        version: &str,
        ws_url: &str,
        timeout_secs: Option<u64>,
        proxy_url: Option<String>,
    ) -> PyResult<Self> {
        let _ = (timeout_secs, proxy_url);
        let api_key = api_key
            .or_else(|| std::env::var("OPENALGO_API_KEY").ok())
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "OpenAlgo API key not provided; set api_key or OPENALGO_API_KEY",
                )
            })?;

        Ok(Self::new(&api_key, host, version, ws_url))
    }

    #[pyo3(name = "connect")]
    fn py_connect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(()) })
    }

    #[pyo3(name = "close")]
    fn py_close<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        pyo3_async_runtimes::tokio::future_into_py(py, async move { Ok(()) })
    }

    #[pyo3(name = "place_order", signature = (strategy, symbol, action, exchange, pricetype, product, quantity, price=None, trigger_price=None))]
    fn py_place_order<'py>(
        &self,
        py: Python<'py>,
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
        price: Option<&str>,
        trigger_price: Option<&str>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let args = OrderArgs::new(
            strategy, symbol, action, exchange, pricetype, product, quantity,
        );
        let price = price.map(str::to_string);
        let trigger_price = trigger_price.map(str::to_string);

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client
                .place_order(
                    &args.strategy,
                    &args.symbol,
                    &args.action,
                    &args.exchange,
                    &args.pricetype,
                    &args.product,
                    &args.quantity,
                    price.as_deref(),
                    trigger_price.as_deref(),
                )
                .await
                .map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "modify_order")]
    fn py_modify_order<'py>(
        &self,
        py: Python<'py>,
        orderid: &str,
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
        price: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let orderid = orderid.to_string();
        let args = OrderArgs::new(
            strategy, symbol, action, exchange, pricetype, product, quantity,
        );
        let price = price.to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client
                .modify_order(
                    &orderid,
                    &args.strategy,
                    &args.symbol,
                    &args.action,
                    &args.exchange,
                    &args.pricetype,
                    &args.product,
                    &args.quantity,
                    &price,
                )
                .await
                .map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "cancel_order")]
    fn py_cancel_order<'py>(
        &self,
        py: Python<'py>,
        orderid: &str,
        strategy: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let orderid = orderid.to_string();
        let strategy = strategy.to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client
                .cancel_order(&orderid, &strategy)
                .await
                .map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "cancel_all_order")]
    fn py_cancel_all_order<'py>(
        &self,
        py: Python<'py>,
        strategy: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let strategy = strategy.to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client
                .cancel_all_order(&strategy)
                .await
                .map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "order_status")]
    fn py_order_status<'py>(
        &self,
        py: Python<'py>,
        orderid: &str,
        strategy: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        let orderid = orderid.to_string();
        let strategy = strategy.to_string();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client
                .order_status(&orderid, &strategy)
                .await
                .map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "funds")]
    fn py_funds<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client.funds().await.map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "orderbook")]
    fn py_orderbook<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client.orderbook().await.map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "tradebook")]
    fn py_tradebook<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client.tradebook().await.map_err(to_pyvalue_err)
        })
    }

    #[pyo3(name = "positionbook")]
    fn py_positionbook<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let client = self.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            client.positionbook().await.map_err(to_pyvalue_err)
        })
    }
}

#[derive(Debug)]
struct OrderArgs {
    strategy: String,
    symbol: String,
    action: String,
    exchange: String,
    pricetype: String,
    product: String,
    quantity: String,
}

impl OrderArgs {
    fn new(
        strategy: &str,
        symbol: &str,
        action: &str,
        exchange: &str,
        pricetype: &str,
        product: &str,
        quantity: &str,
    ) -> Self {
        Self {
            strategy: strategy.to_string(),
            symbol: symbol.to_string(),
            action: action.to_string(),
            exchange: exchange.to_string(),
            pricetype: pricetype.to_string(),
            product: product.to_string(),
            quantity: quantity.to_string(),
        }
    }
}
