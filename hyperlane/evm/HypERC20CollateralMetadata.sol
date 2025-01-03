// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

// TODO: I can't get remappings to work, so importing using full paths...
import {TokenMessage} from "../../dependencies/hyperlane-monorepo-0.0.0/solidity/contracts/token/libs/TokenMessage.sol";
import {HypERC20Collateral} from "../../dependencies/hyperlane-monorepo-0.0.0/solidity/contracts/token/HypERC20Collateral.sol";

contract HypERC20CollateralMetadata is HypERC20Collateral {
    constructor(
        address erc20,
        address _mailbox
    ) HypERC20Collateral(erc20, _mailbox) {}

    /**
     * @notice Transfers `_amount` token to `_recipient` on `_destination` domain
     * with a specified token metadata.
     */
    function transferRemote(
        uint32 _destination,
        bytes32 _recipient,
        uint256 _amount,
        bytes calldata _tokenMetadata
    ) external payable returns (bytes32 messageId) {
        return
            _transferRemote(
                _destination,
                _recipient,
                _amount,
                msg.value,
                _tokenMetadata,
                _GasRouter_hookMetadata(_destination),
                address(hook)
            );
    }

    /**
     * @notice Transfers `_amount` token to `_recipient` on `_destination` domain
     * with a specified token metadata and a hook.
     */
    function transferRemote(
        uint32 _destination,
        bytes32 _recipient,
        uint256 _amount,
        bytes calldata _tokenMetadata,
        bytes calldata _hookMetadata,
        address _hook
    ) external payable returns (bytes32 messageId) {
        return
            _transferRemote(
                _destination,
                _recipient,
                _amount,
                msg.value,
                _tokenMetadata,
                _hookMetadata,
                _hook
            );
    }

    function _transferRemote(
        uint32 _destination,
        bytes32 _recipient,
        uint256 _amount,
        uint256 _value,
        bytes calldata _tokenMetadata,
        bytes memory _hookMetadata,
        address _hook
    ) internal returns (bytes32 messageId) {
        _transferFromSender(_amount);

        bytes memory _tokenMessage = TokenMessage.format(
            _recipient,
            _amount,
            _tokenMetadata
        );

        messageId = _Router_dispatch(
            _destination,
            _value,
            _tokenMessage,
            _hookMetadata,
            _hook
        );

        emit SentTransferRemote(_destination, _recipient, _amount);
    }
}
