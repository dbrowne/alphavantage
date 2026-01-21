#!/bin/bash
############################################################
# Help                                                     #
############################################################
Help()
{
   # Display Help
   echo "Populates Intraday Prices for all Symbols from the year passed to 2026."
   echo
   echo "Syntax: populate_intraday_from_year.sh [-h|-y]"
   echo "options:"
   echo "h     These help details."
   echo "y     The year to start from (default '2005')."
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
Year="2005"

############################################################
# Process the input options. Add options as needed.        #
############################################################
# Get the options
while getopts ":hy:" option; do
   case $option in
      h) # Display Help
         Help
         exit;;
      y) # Database User
        Year=$OPTARG;;
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

if ((Year < 2005 || Year > 2026)); then
  echo "Initial year is out-of-bounds: $((Year)), reverting to 2005."
  Year=2005
fi

for year in $(seq "${Year}" 2026);
do
  for month in $(seq -f "%02g" 1 12);
  do
    cargo run load intraday --update --extended-hours --update-symbols --concurrent=5 --api-delay=250 --force-refresh --month="${year}"-"${month}"
  done
done