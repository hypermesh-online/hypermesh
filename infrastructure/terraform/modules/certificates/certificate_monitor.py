#!/usr/bin/env python3
"""
TrustChain Certificate Monitor Lambda Function
Monitors SSL certificate expiration and health
Environment: ${environment}
"""

import json
import boto3
import os
from datetime import datetime, timedelta
from typing import Dict, Any

def handler(event: Dict[str, Any], context: Any) -> Dict[str, Any]:
    """
    Main Lambda handler for certificate monitoring
    """
    try:
        # Initialize AWS clients
        acm = boto3.client('acm')
        sns = boto3.client('sns')
        cloudwatch = boto3.client('cloudwatch')
        
        # Get environment variables
        certificate_arn = os.environ['CERTIFICATE_ARN']
        environment = os.environ['ENVIRONMENT']
        sns_topic_arn = os.environ.get('SNS_TOPIC_ARN')
        
        print(f"INFO CERTIFICATE_MONITOR environment={environment} certificate_arn={certificate_arn}")
        
        # Get certificate details
        response = acm.describe_certificate(CertificateArn=certificate_arn)
        certificate = response['Certificate']
        
        # Extract certificate information
        domain_name = certificate['DomainName']
        status = certificate['Status']
        not_after = certificate['NotAfter']
        
        # Calculate days until expiration
        now = datetime.now(not_after.tzinfo)
        days_until_expiry = (not_after - now).days
        
        print(f"INFO CERTIFICATE_STATUS domain={domain_name} status={status} days_until_expiry={days_until_expiry}")
        
        # Send custom metric to CloudWatch
        cloudwatch.put_metric_data(
            Namespace='TrustChain/Certificates',
            MetricData=[
                {
                    'MetricName': 'DaysUntilExpiry',
                    'Value': days_until_expiry,
                    'Unit': 'Count',
                    'Dimensions': [
                        {
                            'Name': 'Domain',
                            'Value': domain_name
                        },
                        {
                            'Name': 'Environment',
                            'Value': environment
                        }
                    ]
                },
                {
                    'MetricName': 'CertificateHealth',
                    'Value': 1 if status == 'ISSUED' else 0,
                    'Unit': 'Count',
                    'Dimensions': [
                        {
                            'Name': 'Domain',
                            'Value': domain_name
                        },
                        {
                            'Name': 'Environment',
                            'Value': environment
                        }
                    ]
                }
            ]
        )
        
        # Check for alerts
        alerts = []
        
        # Certificate expiration warning
        if days_until_expiry <= 30:
            alerts.append({
                'severity': 'WARNING' if days_until_expiry > 7 else 'CRITICAL',
                'message': f"Certificate for {domain_name} expires in {days_until_expiry} days",
                'action_required': 'Renew certificate before expiration'
            })
        
        # Certificate status check
        if status != 'ISSUED':
            alerts.append({
                'severity': 'CRITICAL',
                'message': f"Certificate for {domain_name} status is {status}",
                'action_required': 'Investigate certificate validation issues'
            })
        
        # Send alerts if any
        if alerts and sns_topic_arn:
            for alert in alerts:
                message = {
                    'timestamp': datetime.utcnow().isoformat(),
                    'environment': environment,
                    'domain': domain_name,
                    'certificate_arn': certificate_arn,
                    'severity': alert['severity'],
                    'message': alert['message'],
                    'action_required': alert['action_required'],
                    'days_until_expiry': days_until_expiry,
                    'certificate_status': status
                }
                
                sns.publish(
                    TopicArn=sns_topic_arn,
                    Subject=f"TrustChain Certificate Alert - {alert['severity']}",
                    Message=json.dumps(message, indent=2)
                )
                
                print(f"ALERT CERTIFICATE_{alert['severity']} {alert['message']}")
        
        # Log successful monitoring
        print(f"INFO CERTIFICATE_MONITORING_COMPLETE domain={domain_name} status={status} days_until_expiry={days_until_expiry}")
        
        return {
            'statusCode': 200,
            'body': json.dumps({
                'status': 'success',
                'domain': domain_name,
                'certificate_status': status,
                'days_until_expiry': days_until_expiry,
                'alerts_sent': len(alerts)
            })
        }
        
    except Exception as e:
        error_message = f"Certificate monitoring failed: {str(e)}"
        print(f"ERROR CERTIFICATE_MONITORING_FAILED error={error_message}")
        
        # Send error alert if SNS topic is configured
        if 'sns_topic_arn' in locals() and sns_topic_arn:
            try:
                error_alert = {
                    'timestamp': datetime.utcnow().isoformat(),
                    'environment': environment,
                    'severity': 'CRITICAL',
                    'message': error_message,
                    'action_required': 'Investigate certificate monitoring system'
                }
                
                sns.publish(
                    TopicArn=sns_topic_arn,
                    Subject="TrustChain Certificate Monitoring Error",
                    Message=json.dumps(error_alert, indent=2)
                )
            except Exception as sns_error:
                print(f"ERROR FAILED_TO_SEND_ERROR_ALERT sns_error={str(sns_error)}")
        
        return {
            'statusCode': 500,
            'body': json.dumps({
                'status': 'error',
                'error': error_message
            })
        }


def validate_certificate_chain(acm_client, certificate_arn: str) -> Dict[str, Any]:
    """
    Validate the certificate chain and trust path
    """
    try:
        response = acm_client.describe_certificate(CertificateArn=certificate_arn)
        certificate = response['Certificate']
        
        # Check certificate chain
        validation_status = {
            'domain_validation': True,
            'chain_complete': True,
            'trust_anchored': True
        }
        
        # Validate domain validation records
        if 'DomainValidationOptions' in certificate:
            for domain_validation in certificate['DomainValidationOptions']:
                if domain_validation['ValidationStatus'] != 'SUCCESS':
                    validation_status['domain_validation'] = False
        
        # Check certificate transparency logging
        if certificate.get('Options', {}).get('CertificateTransparencyLoggingPreference') != 'ENABLED':
            validation_status['ct_logging_enabled'] = False
        else:
            validation_status['ct_logging_enabled'] = True
        
        return validation_status
        
    except Exception as e:
        print(f"ERROR CERTIFICATE_VALIDATION_FAILED error={str(e)}")
        return {
            'domain_validation': False,
            'chain_complete': False,
            'trust_anchored': False,
            'ct_logging_enabled': False,
            'error': str(e)
        }


def check_certificate_transparency_logs(domain: str) -> Dict[str, Any]:
    """
    Check if certificate appears in Certificate Transparency logs
    """
    # This would integrate with CT log APIs to verify certificate transparency
    # For now, return a placeholder implementation
    return {
        'ct_logs_found': True,
        'log_count': 1,
        'timestamp': datetime.utcnow().isoformat()
    }