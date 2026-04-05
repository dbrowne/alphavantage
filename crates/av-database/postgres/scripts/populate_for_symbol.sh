#!/bin/bash
############################################################
# Help                                                     #
############################################################
Help()
{
   # Display Help
   echo "Populates Daily Prices and Intraday Prices for Symbols that were 'missing' and are no longer."
   echo
   echo "Syntax: populate_for_symbol.sh [-c|-d|-h|-i|-p|-s|-u]"
   echo "options:"
   echo "c     The database password (default 'dev_pw')."
   echo "d     The database (default 'sec_master')."
   echo "h     These help details."
   echo "i     The database hostname (default 'localhost')."
   echo "p     The database post (default '5432')."
   echo "s     The Symbol to populate."
   echo "u     The database username (default 'ts_user')."
   echo
}

############################################################
# Echoes the error and help info, then exits.              #
############################################################
ErrorEcho() {
  echo
  echo "${1}"
  echo
  Help
  exit;
}

############################################################
############################################################
# Main program                                             #
############################################################
############################################################

# Set variables
Database="sec_master"
DBPassword="dev_pw"
DBHost="localhost"
DBPort="5432"
DBUSer="ts_user"
Symbol=""
Year=""
Month=""
FirstYear=true

############################################################
# Process the input options. Add options as needed.        #
############################################################
# Get the options
while getopts ":c:d:hi:p:s:u:" option; do
   case $option in
      c) # Database Password/Credential
        DBPassword=$OPTARG;;
      d) # Database
        Database=$OPTARG;;
      h) # Display Help
         Help
         exit;;
      i) # Database Host (IP Address or Hostname)
        DBHost=$OPTARG;;
      p) # Database Port
        DBPort=$OPTARG;;
      s) # Symbol
        Symbol=$OPTARG;;
      u) # Database User
        DBUSer=$OPTARG;;
     \?) # Invalid option
         ErrorEcho "Error: Invalid option";;
   esac
done

if [[ -z "$ALPHA_VANTAGE_API_KEY" ]]; then
  ErrorEcho "Missing 'ALPHA_VANTAGE_API_KEY' environment variable, please set prior to running this script."
fi

if [[ -z "$DATABASE_URL" ]]; then
  ErrorEcho "Missing 'DATABASE_URL' environment variable, please set prior to running this script."
fi

export RUSTFLAGS="-Awarnings"

cargo run load daily --outputsize=full --concurrent=10 --api-delay=100 --symbol="${Symbol}"

# Query database to find the first year and month this Symbol started getting Summary Prices, and use that as a starting date for Intraday Prices
Year=$(psql "host=${DBHost} port=${DBPort} dbname=${Database} user=${DBUSer} password=${DBPassword}" -c "SELECT MIN(EXTRACT(YEAR FROM date)) FROM summaryprices WHERE symbol = '${Symbol}';" -t)
Month=$(psql "host=${DBHost} port=${DBPort} dbname=${Database} user=${DBUSer} password=${DBPassword}" -c "SELECT MIN(EXTRACT(MONTH FROM date)) FROM summaryprices WHERE symbol = '${Symbol}' AND date >= '$((Year))-01-01' AND date <= '$((Year))-12-31';" -t)

echo "+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++"

if ((Year < 2005)); then
  echo "Initial date is out-of-bounds: $((Month))/$((Year)), reverting to 1/2005."
  Year=2005
  Month=1
fi

echo "Extracting Intraday Prices from Symbol '${Symbol}' from $((Month))/$((Year))."
echo "+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++"

for year in $(seq $((Year)) 2025);
do
  if [[ "$FirstYear" = true ]]; then
    for month in $(seq -f "%02g" $((Month)) 12);
    do
      cargo run load intraday --update --extended-hours --update-symbols --concurrent=5 --api-delay=250 --force-refresh --month="${year}"-"${month}" --symbol="${Symbol}"
    done
    FirstYear=false
  else
    for month in $(seq -f "%02g" 1 12);
    do
      cargo run load intraday --update --extended-hours --update-symbols --concurrent=5 --api-delay=250 --force-refresh --month="${year}"-"${month}" --symbol="${Symbol}"
    done
  fi
done

cargo run load intraday --force-refresh --concurrent=5 --api-delay=250 --symbol="${Symbol}"