#!/bin/bash

HOMEBASE_PATH="/usr/local/bin/homebase"
HOMEBASE_OPTS="-t 9d88d111-27d2-4cb5-975b-94e0b034c38a.example.com -4 -z -c /tmp/homebase_mikrotik"

MIKROTIK_USER="admin"
MIKROTIK_SSH_KEY="/root/.ssh/homebase_mikrotik_id_rsa"
MIKROTIK_HOST="192.168.0.1"
MIKROTIK_ADDRESS_LIST="homebase"

# Get homebase CIDRs
MIKROTIK_ADDRESS_LIST_CMDS=""
if cidrs=$($HOMEBASE_PATH $HOMEBASE_OPTS 2>/dev/null) && [ $(echo "$cidrs" | wc -l) -ge 1 ] ; then

  # Get current SHA256 of both homebase CIDRs and MikroTik CIDRs.
  HOMEBASE_CIDRS_SHA256=$(echo "$cidrs" | sha256sum | awk '{print $1}')
  MIKROTIK_CIDRS_SHA256=$(echo "/ip firewall address-list export where list=${MIKROTIK_ADDRESS_LIST}" | ssh -q -oStrictHostKeyChecking=accept-new -oIdentityFile=$MIKROTIK_SSH_KEY $MIKROTIK_USER@$MIKROTIK_HOST | egrep -o 'address=[a-fA-F0-9.:/]+' | awk -F'=' '{print $2}' | sed -re 's;^([0-9.]+)$;\1/32;' -e 's;^([a-fA-F0-9:]+)$;\1/128;' | sha256sum | awk '{print $1}')

  # Compare SHA256 checksums.
  if [ "x$HOMEBASE_CIDRS_SHA256" != "x$MIKROTIK_CIDRS_SHA256" ] ; then

    # Build string of MikroTik commands for adding/refreshing the address list.
    MIKROTIK_ADDRESS_LIST_CMDS="/ip firewall address-list remove [/ip firewall address-list find where list=\"${MIKROTIK_ADDRESS_LIST}\"]\n"
    for cidr in $cidrs ; do
      MIKROTIK_ADDRESS_LIST_CMDS="${MIKROTIK_ADDRESS_LIST_CMDS}/ip firewall address-list add address=$cidr list=${MIKROTIK_ADDRESS_LIST}\n"
    done

  fi

  # Add/refresh the MikroTik address list.
  if [ $(echo -e "$MIKROTIK_ADDRESS_LIST_CMDS" | wc -l) -gt 1 ] ; then
    echo -e "$MIKROTIK_ADDRESS_LIST_CMDS" | ssh -q -oStrictHostKeyChecking=accept-new -oIdentityFile=$MIKROTIK_SSH_KEY $MIKROTIK_USER@$MIKROTIK_HOST
  fi

fi

