// SPDX-License-Identifier: MIT
pragma solidity ^0.8.9;

import "../interfaces/IFlashloan.sol";

library RouteUtils {
    function getInitialToken(IFlashloan.Route memory route)
        internal
        pure
        returns (address)
    {
        return route.hops[0].path[0];
    }
}
